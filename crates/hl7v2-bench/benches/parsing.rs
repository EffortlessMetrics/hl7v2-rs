//! Benchmarks for HL7 v2 parsing performance

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hl7v2_parser::parse;
use hl7v2_writer::write;
use std::hint::black_box;

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250128152312||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250128152312\r".to_string()
}

/// Create a larger message with repeated segments for benchmarking
fn create_large_message() -> String {
    let base_message = create_sample_message();
    let mut large_message = String::new();

    for _ in 0..10 {
        large_message.push_str(&base_message);
    }

    large_message
}

/// Benchmark parsing performance
fn bench_parse(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("parse_basic", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark parsing performance for large messages
fn bench_parse_large(c: &mut Criterion) {
    let message = create_large_message();
    let bytes = message.as_bytes();

    c.bench_function("parse_large", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark writing performance
fn bench_write(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("write_basic", |b| {
        b.iter(|| {
            let result = write(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark parsing performance with varying message sizes
fn bench_parse_sizes(c: &mut Criterion) {
    let base_message = create_sample_message();
    let mut group = c.benchmark_group("parse_sizes");

    for size in [1, 2, 5, 10, 20].iter() {
        let mut message = String::new();
        for _ in 0..*size {
            message.push_str(&base_message);
        }
        let bytes = message.as_bytes();

        group.bench_with_input(BenchmarkId::from_parameter(size), bytes, |b, bytes| {
            b.iter(|| {
                let result = parse(black_box(bytes));
                let _ = black_box(result);
            });
        });
    }
    group.finish();
}

criterion_group!(
    parsing_benches,
    bench_parse,
    bench_parse_large,
    bench_write,
    bench_parse_sizes
);

criterion_main!(parsing_benches);
