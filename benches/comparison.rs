use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lotus::metrics::{
    elias_delta_decode, elias_delta_encode, elias_gamma_decode, elias_gamma_encode, leb128_decode,
    leb128_encode, standard_workloads, vlq_decode, vlq_encode,
};
use lotus::{
    BitReader, BitWriter, RECOMMENDED_PROFILES, lotus_decode_from_reader, lotus_encode_into_writer,
    lotus_encoded_bit_len,
};

fn profile_covers(values: &[u64], j: usize, d: usize) -> bool {
    values
        .iter()
        .all(|&value| lotus_encoded_bit_len(value, j, d).is_ok())
}

fn bench_encode(c: &mut Criterion, name: &str, values: &[u64]) {
    let mut group = c.benchmark_group(format!("encode_{name}"));
    group.throughput(Throughput::Elements(values.len() as u64));

    for profile in RECOMMENDED_PROFILES {
        let j = profile.config.jumpstarter_bits;
        let d = profile.config.tiers;
        if !profile_covers(values, j, d) {
            continue;
        }
        group.bench_with_input(BenchmarkId::new("LotusPacked", profile.label), values, |b, v| {
            b.iter(|| {
                let mut writer = BitWriter::new();
                for &value in v {
                    lotus_encode_into_writer(value, j, d, &mut writer).unwrap();
                }
                criterion::black_box(writer.into_bytes());
            });
        });
    }

    group.bench_function("LEB128", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for &value in values {
                total = total.wrapping_add(leb128_encode(value).len());
            }
            criterion::black_box(total);
        });
    });

    group.bench_function("VLQ", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for &value in values {
                total = total.wrapping_add(vlq_encode(value).len());
            }
            criterion::black_box(total);
        });
    });

    group.bench_function("EliasGamma", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for &value in values {
                total = total.wrapping_add(elias_gamma_encode(value).len());
            }
            criterion::black_box(total);
        });
    });

    group.bench_function("EliasDelta", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for &value in values {
                total = total.wrapping_add(elias_delta_encode(value).len());
            }
            criterion::black_box(total);
        });
    });

    group.finish();
}

fn bench_decode(c: &mut Criterion, name: &str, values: &[u64]) {
    let mut group = c.benchmark_group(format!("decode_{name}"));
    group.throughput(Throughput::Elements(values.len() as u64));

    for profile in RECOMMENDED_PROFILES {
        let j = profile.config.jumpstarter_bits;
        let d = profile.config.tiers;
        if !profile_covers(values, j, d) {
            continue;
        }

        let mut writer = BitWriter::new();
        for &value in values {
            lotus_encode_into_writer(value, j, d, &mut writer).unwrap();
        }
        let encoded = writer.into_bytes();

        group.bench_with_input(
            BenchmarkId::new("LotusPacked", profile.label),
            &encoded,
            |b, bytes| {
                b.iter(|| {
                    let mut reader = BitReader::new(bytes);
                    let mut checksum = 0u64;
                    for _ in values {
                        checksum = checksum
                            .wrapping_add(lotus_decode_from_reader(&mut reader, j, d).unwrap().0);
                    }
                    criterion::black_box(checksum);
                });
            },
        );
    }

    let leb = values
        .iter()
        .map(|&value| leb128_encode(value))
        .collect::<Vec<_>>();
    group.bench_function("LEB128", |b| {
        b.iter(|| {
            let mut checksum = 0u64;
            for bytes in &leb {
                checksum = checksum.wrapping_add(leb128_decode(bytes).unwrap().0);
            }
            criterion::black_box(checksum);
        });
    });

    let vlq = values
        .iter()
        .map(|&value| vlq_encode(value))
        .collect::<Vec<_>>();
    group.bench_function("VLQ", |b| {
        b.iter(|| {
            let mut checksum = 0u64;
            for bytes in &vlq {
                checksum = checksum.wrapping_add(vlq_decode(bytes).unwrap().0);
            }
            criterion::black_box(checksum);
        });
    });

    let gamma = values
        .iter()
        .map(|&value| elias_gamma_encode(value))
        .collect::<Vec<_>>();
    group.bench_function("EliasGamma", |b| {
        b.iter(|| {
            let mut checksum = 0u64;
            for bytes in &gamma {
                checksum = checksum.wrapping_add(elias_gamma_decode(bytes).unwrap().0);
            }
            criterion::black_box(checksum);
        });
    });

    let delta = values
        .iter()
        .map(|&value| elias_delta_encode(value))
        .collect::<Vec<_>>();
    group.bench_function("EliasDelta", |b| {
        b.iter(|| {
            let mut checksum = 0u64;
            for bytes in &delta {
                checksum = checksum.wrapping_add(elias_delta_decode(bytes).unwrap().0);
            }
            criterion::black_box(checksum);
        });
    });

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    for workload in standard_workloads() {
        bench_encode(c, workload.name, &workload.values);
        bench_decode(c, workload.name, &workload.values);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
