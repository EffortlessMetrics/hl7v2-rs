//! Benchmarks for HL7 v2 batch file processing performance
//!
//! This benchmark suite profiles batch processing performance:
//! - Batch file parsing (FHS/BHS)
//! - Multi-message batch processing
//! - Batch with different message counts (10, 100, 1000)

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2_batch::parse_batch;
use hl7v2_parser::parse;
use std::hint::black_box;

/// Create a sample HL7 message for batch construction
fn create_sample_message(msg_id: usize) -> String {
    format!(
        "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000||ADT^A01^ADT_A01|MSG{:05}|P|2.5.1\rPID|1||{}^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|I|ICU^101^01||||DOE^JOHN^A^III^^^^MD|||SUR||||||ADM|||||||||||||||||||||||||20250101120000\r",
        msg_id,
        100000 + msg_id
    )
}

/// Create a batch file header (FHS)
fn create_fhs() -> String {
    "FHS|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000\r"
        .to_string()
}

/// Create a batch header (BHS)
fn create_bhs() -> String {
    "BHS|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000\r"
        .to_string()
}

/// Create a batch trailer (BTS)
fn create_bts(count: usize) -> String {
    format!("BTS|{}\r", count)
}

/// Create a file trailer (FTS)
fn create_fts(count: usize) -> String {
    format!("FTS|{}\r", count)
}

/// Create a single batch with specified number of messages
fn create_single_batch(message_count: usize) -> String {
    let mut batch = String::new();

    batch.push_str(&create_bhs());
    for i in 0..message_count {
        batch.push_str(&create_sample_message(i));
    }
    batch.push_str(&create_bts(message_count));

    batch
}

/// Create a file batch with FHS/FTS and multiple batches
fn create_file_batch(messages_per_batch: usize, batch_count: usize) -> String {
    let mut file = String::new();

    file.push_str(&create_fhs());
    for _ in 0..batch_count {
        file.push_str(&create_single_batch(messages_per_batch));
    }
    file.push_str(&create_fts(messages_per_batch * batch_count));

    file
}

/// Create a simple batch (no FHS/FTS) with specified messages
fn create_simple_batch(message_count: usize) -> String {
    let mut batch = String::new();

    for i in 0..message_count {
        batch.push_str(&create_sample_message(i));
    }

    batch
}

/// Benchmark parsing a batch with BHS/BTS
fn bench_parse_single_batch(c: &mut Criterion) {
    let batch = create_single_batch(10);
    let bytes = batch.as_bytes();

    c.bench_function("parse_single_batch_10", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark parsing a file batch with FHS/FTS
fn bench_parse_file_batch(c: &mut Criterion) {
    let file = create_file_batch(10, 2);
    let bytes = file.as_bytes();

    c.bench_function("parse_file_batch_10x2", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark batch parsing with different message counts
fn bench_batch_by_message_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_message_count");

    for count in [10, 100, 1000].iter() {
        let batch = create_single_batch(*count);
        let bytes = batch.as_bytes();

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse_batch", count), count, |b, _| {
            b.iter(|| {
                let result = parse_batch(black_box(bytes));
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark batch parsing with different file sizes
fn bench_batch_by_file_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_file_size");

    // Small batch (10 messages)
    let small = create_single_batch(10);
    let small_bytes = small.as_bytes();
    group.throughput(Throughput::Bytes(small_bytes.len() as u64));
    group.bench_function("small_10_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(small_bytes));
            black_box(result)
        })
    });

    // Medium batch (100 messages)
    let medium = create_single_batch(100);
    let medium_bytes = medium.as_bytes();
    group.throughput(Throughput::Bytes(medium_bytes.len() as u64));
    group.bench_function("medium_100_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(medium_bytes));
            black_box(result)
        })
    });

    // Large batch (1000 messages)
    let large = create_single_batch(1000);
    let large_bytes = large.as_bytes();
    group.throughput(Throughput::Bytes(large_bytes.len() as u64));
    group.bench_function("large_1000_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(large_bytes));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark file batch with multiple nested batches
fn bench_nested_batches(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_batches");

    // 10 batches of 10 messages each
    let nested_10x10 = create_file_batch(10, 10);
    let bytes_10x10 = nested_10x10.as_bytes();
    group.throughput(Throughput::Bytes(bytes_10x10.len() as u64));
    group.bench_function("10_batches_x_10_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(bytes_10x10));
            black_box(result)
        })
    });

    // 5 batches of 20 messages each
    let nested_5x20 = create_file_batch(20, 5);
    let bytes_5x20 = nested_5x20.as_bytes();
    group.throughput(Throughput::Bytes(bytes_5x20.len() as u64));
    group.bench_function("5_batches_x_20_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(bytes_5x20));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark individual message parsing within a batch
fn bench_message_parsing_from_batch(c: &mut Criterion) {
    let batch = create_simple_batch(100);
    let bytes = batch.as_bytes();

    c.bench_function("parse_100_messages_sequential", |b| {
        b.iter(|| {
            // Split by segment terminator and parse each message
            let text = std::str::from_utf8(black_box(bytes)).unwrap();
            let messages: Vec<&str> = text.split("MSH|").filter(|s| !s.is_empty()).collect();

            for msg_content in messages {
                let full_msg = format!("MSH|{}", msg_content);
                let result = parse(full_msg.as_bytes());
                black_box(result);
            }
        })
    });
}

/// Benchmark batch iteration performance
fn bench_batch_iteration(c: &mut Criterion) {
    let batch = create_single_batch(100);
    let bytes = batch.as_bytes();
    let parsed = parse_batch(bytes).expect("Failed to parse batch");

    c.bench_function("iterate_100_message_batch", |b| {
        b.iter(|| {
            let mut count = 0;
            for batch in &parsed.batches {
                for _ in batch.iter_messages() {
                    count += 1;
                }
            }
            black_box(count)
        })
    });
}

/// Benchmark batch with large messages
fn bench_batch_large_messages(c: &mut Criterion) {
    // Create messages with more segments
    fn create_large_message(msg_id: usize) -> String {
        let mut msg = format!(
            "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000||ADT^A01^ADT_A01|MSG{:05}|P|2.5.1\rPID|1||{}^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|I|ICU^101^01||||DOE^JOHN^A^III^^^^MD|||SUR||||||ADM|||||||||||||||||||||||||20250101120000\r",
            msg_id,
            100000 + msg_id
        );

        // Add multiple OBX segments
        for i in 1..=20 {
            msg.push_str(&format!(
                "OBX|{}|NM|OBS{:03}^Observation {}^L||{:.2}|units|||||F\r",
                i,
                i,
                i,
                100.0 + i as f64
            ));
        }

        // Add multiple AL1 segments
        for i in 1..=5 {
            msg.push_str(&format!(
                "ALL|{}|DA|ALL{:03}^Allergen {}^L||Reaction {}||20200101\r",
                i, i, i, i
            ));
        }

        // Add multiple DG1 segments
        for i in 1..=3 {
            msg.push_str(&format!(
                "DG1|{}|ICD10|DX{:03}^Diagnosis {}||20250101||A\r",
                i, i, i
            ));
        }

        msg
    }

    let mut batch = String::new();
    batch.push_str(&create_bhs());
    for i in 0..50 {
        batch.push_str(&create_large_message(i));
    }
    batch.push_str(&create_bts(50));

    let bytes = batch.as_bytes();

    c.bench_function("parse_batch_50_large_messages", |b| {
        b.iter(|| {
            let result = parse_batch(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark memory efficiency of batch parsing
fn bench_batch_memory_efficiency(c: &mut Criterion) {
    let batch = create_single_batch(500);
    let bytes = batch.as_bytes();

    let mut group = c.benchmark_group("batch_memory");

    // Parse and count messages
    group.bench_function("parse_and_count_500", |b| {
        b.iter(|| {
            let parsed = parse_batch(black_box(bytes)).expect("Failed to parse");
            let count = parsed.total_message_count();
            black_box(count)
        })
    });

    // Parse and access first message
    group.bench_function("parse_and_access_first_500", |b| {
        b.iter(|| {
            let parsed = parse_batch(black_box(bytes)).expect("Failed to parse");
            let first_msg = parsed.iter_all_messages().next();
            let segment_count = first_msg.map(|m| m.segments.len()).unwrap_or(0);
            black_box(segment_count)
        })
    });

    group.finish();
}

criterion_group!(
    batch_benches,
    bench_parse_single_batch,
    bench_parse_file_batch,
    bench_batch_by_message_count,
    bench_batch_by_file_size,
    bench_nested_batches,
    bench_message_parsing_from_batch,
    bench_batch_iteration,
    bench_batch_large_messages,
    bench_batch_memory_efficiency,
);

criterion_main!(batch_benches);
