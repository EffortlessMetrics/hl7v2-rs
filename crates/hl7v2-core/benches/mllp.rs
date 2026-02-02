//! Benchmarks for HL7 v2 MLLP functionality

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{parse, parse_mllp, write, write_mllp};

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\r".to_string()
}

/// Benchmark MLLP wrapping
fn bench_mllp_wrap(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");
    let _bytes = write(&parsed);

    c.bench_function("mllp_wrap", |b| {
        b.iter(|| {
            let result = write_mllp(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark MLLP parsing
fn bench_mllp_parse(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");
    let mllp_bytes = write_mllp(&parsed);

    c.bench_function("mllp_parse", |b| {
        b.iter(|| {
            let result = parse_mllp(black_box(&mllp_bytes));
            black_box(result)
        })
    });
}

criterion_group!(mllp_benches, bench_mllp_wrap, bench_mllp_parse);

criterion_main!(mllp_benches);
