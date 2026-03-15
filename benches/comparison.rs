use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lotus::metrics::{elias_delta_len, leb128_encode, standard_workloads};
use lotus::{LOTUS_J2D1, LOTUS_J3D1, lotus_encode_u64};

fn bench_distribution(c: &mut Criterion, name: &str, values: &[u64]) {
    let j2_values: Vec<u64> = values
        .iter()
        .copied()
        .filter(|v| lotus_encode_u64(*v, LOTUS_J2D1.0, LOTUS_J2D1.1).is_ok())
        .collect();
    let j3_values: Vec<u64> = values
        .iter()
        .copied()
        .filter(|v| lotus_encode_u64(*v, LOTUS_J3D1.0, LOTUS_J3D1.1).is_ok())
        .collect();

    let mut group = c.benchmark_group(format!("lotus_vs_leb128_{name}"));
    group.bench_function(BenchmarkId::new("Lotus J2D1", name), |b| {
        b.iter(|| {
            for v in &j2_values {
                let _ = lotus_encode_u64(*v, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
            }
        });
    });
    group.bench_function(BenchmarkId::new("Lotus J3D1", name), |b| {
        b.iter(|| {
            for v in &j3_values {
                let _ = lotus_encode_u64(*v, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
            }
        });
    });
    group.bench_function(BenchmarkId::new("LEB128", name), |b| {
        b.iter(|| {
            for v in values {
                let _ = leb128_encode(*v);
            }
        });
    });
    group.bench_function(BenchmarkId::new("EliasDelta", name), |b| {
        b.iter(|| {
            for v in values {
                let _ = elias_delta_len(*v);
            }
        });
    });
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    for workload in standard_workloads() {
        bench_distribution(c, workload.name, &workload.values);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
