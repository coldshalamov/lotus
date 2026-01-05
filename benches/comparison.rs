use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lotus::{LOTUS_J2D1, LOTUS_J3D1, lotus_encode_u64};

fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
    out
}

fn elias_delta_len(mut value: u64) -> usize {
    let mut len = 1;
    let mut bits = 64 - value.leading_zeros() as usize;
    while bits > 1 {
        len += bits;
        bits = 64 - (bits as u64).leading_zeros() as usize;
    }
    len
}

fn bench_distribution(c: &mut Criterion, name: &str, values: Vec<u64>) {
    let mut group = c.benchmark_group(format!("lotus_vs_leb128_{name}"));
    group.bench_function(BenchmarkId::new("Lotus J2D1", name), |b| {
        b.iter(|| {
            for v in &values {
                let _ = lotus_encode_u64(*v, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
            }
        });
    });
    group.bench_function(BenchmarkId::new("Lotus J3D1", name), |b| {
        b.iter(|| {
            for v in &values {
                let _ = lotus_encode_u64(*v, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
            }
        });
    });
    group.bench_function(BenchmarkId::new("LEB128", name), |b| {
        b.iter(|| {
            for v in &values {
                let _ = leb128_encode(*v);
            }
        });
    });
    group.bench_function(BenchmarkId::new("EliasDelta", name), |b| {
        b.iter(|| {
            for v in &values {
                let _ = elias_delta_len(*v);
            }
        });
    });
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_distribution(c, "small", (0u64..=255).collect());
    bench_distribution(c, "medium", (0u64..=1_000_000).step_by(10_000).collect());
    bench_distribution(
        c,
        "large32",
        (0u64..=4_000_000_000).step_by(25_000_000).collect(),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
