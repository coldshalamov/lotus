use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lotus::metrics::{
    elias_delta_decode, elias_delta_encode, elias_gamma_decode, elias_gamma_encode, leb128_decode,
    leb128_encode, standard_workloads, vlq_decode, vlq_encode,
};
use lotus::{LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1, lotus_decode_u64, lotus_encode_u64};

/// Lotus configurations benchmarked. Each is `(label, j, d)`.
const LOTUS_CONFIGS: &[(&str, usize, usize)] = &[
    ("J1D2", LOTUS_J1D2.0, LOTUS_J1D2.1),
    ("J2D1", LOTUS_J2D1.0, LOTUS_J2D1.1),
    ("J3D1", LOTUS_J3D1.0, LOTUS_J3D1.1),
    ("J3D2", 3, 2),
];

/// Values from `values` that Lotus `(j, d)` can represent, in source order.
fn lotus_encodable(values: &[u64], j: usize, d: usize) -> Vec<u64> {
    values
        .iter()
        .copied()
        .filter(|v| lotus_encode_u64(*v, j, d).is_ok())
        .collect()
}

// ---------------------------------------------------------------------------
// Encode throughput
// ---------------------------------------------------------------------------

fn bench_encode(c: &mut Criterion, name: &str, values: &[u64]) {
    let mut group = c.benchmark_group(format!("encode_{name}"));
    group.throughput(Throughput::Elements(values.len() as u64));

    for &(label, j, d) in LOTUS_CONFIGS {
        let encodable = lotus_encodable(values, j, d);
        if encodable.is_empty() {
            continue;
        }
        group.bench_with_input(BenchmarkId::new("Lotus", label), &encodable, |b, v| {
            b.iter(|| {
                let mut acc = 0u64;
                for &x in v {
                    acc = acc.wrapping_add(lotus_encode_u64(x, j, d).unwrap().len() as u64);
                }
                criterion::black_box(acc);
            });
        });
    }

    group.bench_function("LEB128", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for &x in values {
                acc = acc.wrapping_add(leb128_encode(x).len() as u64);
            }
            criterion::black_box(acc);
        });
    });

    group.bench_function("VLQ", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for &x in values {
                acc = acc.wrapping_add(vlq_encode(x).len() as u64);
            }
            criterion::black_box(acc);
        });
    });

    group.bench_function("EliasGamma", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for &x in values {
                acc = acc.wrapping_add(elias_gamma_encode(x).len() as u64);
            }
            criterion::black_box(acc);
        });
    });

    group.bench_function("EliasDelta", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for &x in values {
                acc = acc.wrapping_add(elias_delta_encode(x).len() as u64);
            }
            criterion::black_box(acc);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Decode throughput
// ---------------------------------------------------------------------------

fn bench_decode(c: &mut Criterion, name: &str, values: &[u64]) {
    let mut group = c.benchmark_group(format!("decode_{name}"));
    group.throughput(Throughput::Elements(values.len() as u64));

    // Lotus: pre-encode each value into its own byte buffer, then decode them.
    for &(label, j, d) in LOTUS_CONFIGS {
        let encodable = lotus_encodable(values, j, d);
        if encodable.is_empty() {
            continue;
        }
        let encoded: Vec<Vec<u8>> = encodable
            .iter()
            .map(|&v| lotus_encode_u64(v, j, d).unwrap())
            .collect();
        group.bench_with_input(BenchmarkId::new("Lotus", label), &encoded, |b, enc| {
            b.iter(|| {
                let mut acc = 0u64;
                for bytes in enc {
                    acc = acc.wrapping_add(lotus_decode_u64(bytes, j, d).unwrap().0);
                }
                criterion::black_box(acc);
            });
        });
    }

    // LEB128
    {
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| leb128_encode(v)).collect();
        group.bench_function("LEB128", |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for bytes in &encoded {
                    acc = acc.wrapping_add(leb128_decode(bytes).unwrap().0);
                }
                criterion::black_box(acc);
            });
        });
    }

    // VLQ
    {
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| vlq_encode(v)).collect();
        group.bench_function("VLQ", |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for bytes in &encoded {
                    acc = acc.wrapping_add(vlq_decode(bytes).unwrap().0);
                }
                criterion::black_box(acc);
            });
        });
    }

    // Elias gamma (max representable value is u64::MAX - 1)
    {
        let gamma_values: Vec<u64> = values.iter().copied().map(|v| v % u64::MAX).collect();
        let encoded: Vec<Vec<u8>> = gamma_values
            .iter()
            .map(|&v| elias_gamma_encode(v))
            .collect();
        group.bench_function("EliasGamma", |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for bytes in &encoded {
                    acc = acc.wrapping_add(elias_gamma_decode(bytes).unwrap().0);
                }
                criterion::black_box(acc);
            });
        });
    }

    // Elias delta (max representable value is u64::MAX - 1)
    {
        let delta_values: Vec<u64> = values.iter().copied().map(|v| v % u64::MAX).collect();
        let encoded: Vec<Vec<u8>> = delta_values
            .iter()
            .map(|&v| elias_delta_encode(v))
            .collect();
        group.bench_function("EliasDelta", |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for bytes in &encoded {
                    acc = acc.wrapping_add(elias_delta_decode(bytes).unwrap().0);
                }
                criterion::black_box(acc);
            });
        });
    }

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
