//! Benchmarks for HL7 v2 parsing performance

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use hl7v2_core::{parse, write};

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250101000000\r".to_string()
}

/// Create a larger message with repeated segments for benchmarking
fn create_large_message() -> String {
    let base_message = create_sample_message();
    let mut large_message = String::new();

    // Repeat the message 10 times to create a larger message
    for _ in 0..10 {
        large_message.push_str(&base_message);
    }

    large_message
}

/// Benchmark parsing a small HL7 message
fn bench_parse_small_message(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("parse_small_message", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark parsing a large HL7 message
fn bench_parse_large_message(c: &mut Criterion) {
    let message = create_large_message();
    let bytes = message.as_bytes();

    c.bench_function("parse_large_message", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark writing HL7 messages
fn bench_write_message(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("write_message", |b| {
        b.iter(|| {
            let result = write(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark parsing multiple messages
fn bench_parse_multiple_messages(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("parse_multiple");
    for num_messages in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_messages),
            num_messages,
            |b, &num_messages| {
                b.iter(|| {
                    for _ in 0..num_messages {
                        let result = parse(black_box(bytes));
                        let _ = black_box(result);
                    }
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_small_message,
    bench_parse_large_message,
    bench_write_message,
    bench_parse_multiple_messages
);

criterion_main!(benches);
