//! Memory usage benchmarks for HL7 v2 parsing

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use hl7v2_core::{normalize, parse, parse_mllp, wrap_mllp, write, write_mllp};

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

/// Benchmark memory usage for parsing
fn bench_memory_parse(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("memory_parse", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark memory usage for parsing large messages
fn bench_memory_parse_large(c: &mut Criterion) {
    let message = create_large_message();
    let bytes = message.as_bytes();

    c.bench_function("memory_parse_large", |b| {
        b.iter(|| {
            let result = parse(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark memory usage for writing
fn bench_memory_write(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("memory_write", |b| {
        b.iter(|| {
            let result = write(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark memory usage for MLLP parsing
fn bench_memory_parse_mllp(c: &mut Criterion) {
    let message = create_sample_message();
    let mllp_bytes = wrap_mllp(message.as_bytes());

    c.bench_function("memory_parse_mllp", |b| {
        b.iter(|| {
            let result = parse_mllp(black_box(&mllp_bytes));
            black_box(result)
        })
    });
}

/// Benchmark memory usage for MLLP writing
fn bench_memory_write_mllp(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("memory_write_mllp", |b| {
        b.iter(|| {
            let result = write_mllp(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark memory usage for normalization
fn bench_memory_normalize(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("memory_normalize", |b| {
        b.iter(|| {
            let result = normalize(black_box(bytes), false);
            black_box(result)
        })
    });
}

criterion_group!(
    memory_benches,
    bench_memory_parse,
    bench_memory_parse_large,
    bench_memory_write,
    bench_memory_parse_mllp,
    bench_memory_write_mllp,
    bench_memory_normalize
);

criterion_main!(memory_benches);
