use codio_content_id::ContentId;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

fn bench_cid_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cid_creation");

    for size in [1024, 10_240, 102_400, 1_048_576].iter() {
        let content = vec![0u8; *size];
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ContentId::new(black_box(&content)));
        });
    }

    group.finish();
}

fn bench_cid_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("cid_verification");

    for size in [1024, 10_240, 102_400, 1_048_576].iter() {
        let content = vec![0u8; *size];
        let cid = ContentId::new(&content);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| cid.verify(black_box(&content)));
        });
    }

    group.finish();
}

fn bench_cid_string_parsing(c: &mut Criterion) {
    let content = b"Test content for parsing benchmark";
    let cid = ContentId::new(content);
    let cid_str = cid.as_str();

    c.bench_function("cid_from_str", |b| {
        b.iter(|| ContentId::from_str(black_box(cid_str)));
    });
}

fn bench_cid_hash_comparison(c: &mut Criterion) {
    let content1 = b"Content for comparison benchmark";
    let content2 = b"Content for comparison benchmark";
    let cid1 = ContentId::new(content1);
    let cid2 = ContentId::new(content2);

    c.bench_function("cid_equality", |b| {
        b.iter(|| black_box(&cid1) == black_box(&cid2));
    });
}

fn bench_small_content(c: &mut Criterion) {
    let content = b"Small";
    c.bench_function("small_content_10bytes", |b| {
        b.iter(|| ContentId::new(black_box(content)));
    });
}

fn bench_medium_content(c: &mut Criterion) {
    let content = vec![0u8; 1024]; // 1KB
    c.bench_function("medium_content_1kb", |b| {
        b.iter(|| ContentId::new(black_box(&content)));
    });
}

fn bench_large_content(c: &mut Criterion) {
    let content = vec![0u8; 1_048_576]; // 1MB
    c.bench_function("large_content_1mb", |b| {
        b.iter(|| ContentId::new(black_box(&content)));
    });
}

criterion_group!(
    benches,
    bench_cid_creation,
    bench_cid_verification,
    bench_cid_string_parsing,
    bench_cid_hash_comparison,
    bench_small_content,
    bench_medium_content,
    bench_large_content
);
criterion_main!(benches);
