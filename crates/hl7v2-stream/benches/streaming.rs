//! Benchmarks for the hl7v2-stream crate.
//!
//! These benchmarks measure:
//! - Throughput (bytes/second)
//! - Memory efficiency
//! - Parsing performance for various message sizes

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hl7v2_stream::{Event, StreamParser};
use std::io::{BufReader, Cursor};

/// Generate a large HL7 message with specified number of segments
fn generate_large_message(segment_count: usize) -> String {
    let mut msg = String::from("MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\r");

    for i in 0..segment_count {
        msg.push_str(&format!(
            "ZXX|segment_{}|field1|field2|field3|field4|field5\r",
            i
        ));
    }

    msg
}

/// Generate a message with long fields
fn generate_message_with_long_field(field_length: usize) -> String {
    let long_field = "X".repeat(field_length);
    format!(
        "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||{}||Name\r",
        long_field
    )
}

/// Generate multiple messages
fn generate_multiple_messages(count: usize) -> String {
    let mut combined = String::new();

    for i in 0..count {
        combined.push_str(&format!(
            "MSH|^~\\&|App{}|Fac|||20250101||ADT^A01|{}|P|2.5\rPID|1||MRN{}||Patient\r",
            i, i, i
        ));
    }

    combined
}

/// Parse a message and return the event count
fn parse_and_count(msg: &str) -> usize {
    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let mut count = 0;
    while let Ok(Some(event)) = parser.next_event() {
        count += 1;
        black_box(event);
    }
    count
}

/// Parse a message and collect all events
fn parse_and_collect(msg: &str) -> Vec<Event> {
    let cursor = Cursor::new(msg.as_bytes());
    let buf_reader = BufReader::new(cursor);
    let mut parser = StreamParser::new(buf_reader);

    let mut events = Vec::new();
    while let Ok(Some(event)) = parser.next_event() {
        events.push(event);
    }
    events
}

// =============================================================================
// Throughput Benchmarks
// =============================================================================

fn bench_throughput_small_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_small");

    for size in [1, 5, 10, 50].iter() {
        let msg = generate_multiple_messages(*size);
        let bytes = msg.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("messages", size), &msg, |b, msg| {
            b.iter(|| parse_and_count(black_box(msg)));
        });
    }

    group.finish();
}

fn bench_throughput_large_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_large");

    for segments in [100, 500, 1000, 5000].iter() {
        let msg = generate_large_message(*segments);
        let bytes = msg.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("segments", segments), &msg, |b, msg| {
            b.iter(|| parse_and_count(black_box(msg)));
        });
    }

    group.finish();
}

// =============================================================================
// Field Size Benchmarks
// =============================================================================

fn bench_field_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_sizes");

    for field_size in [100, 1_000, 10_000, 100_000].iter() {
        let msg = generate_message_with_long_field(*field_size);
        let bytes = msg.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("bytes", field_size), &msg, |b, msg| {
            b.iter(|| parse_and_count(black_box(msg)));
        });
    }

    group.finish();
}

// =============================================================================
// Parsing Operation Benchmarks
// =============================================================================

fn bench_parse_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_operations");

    let small_msg = "MSH|^~\\&|App|Fac|||20250101||ADT^A01|123|P|2.5\rPID|1||123||Name\r";
    let medium_msg = generate_large_message(50);
    let large_msg = generate_large_message(500);

    group.bench_function("parse_small", |b| {
        b.iter(|| parse_and_collect(black_box(small_msg)));
    });

    group.bench_function("parse_medium", |b| {
        b.iter(|| parse_and_collect(black_box(&medium_msg)));
    });

    group.bench_function("parse_large", |b| {
        b.iter(|| parse_and_collect(black_box(&large_msg)));
    });

    group.finish();
}

// =============================================================================
// Incremental vs Bulk Benchmarks
// =============================================================================

fn bench_incremental_vs_bulk(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_vs_bulk");

    let msg = generate_large_message(1000);

    // Incremental parsing (count events as we go)
    group.bench_function("incremental_count", |b| {
        b.iter(|| {
            let cursor = Cursor::new(msg.as_bytes());
            let buf_reader = BufReader::new(cursor);
            let mut parser = StreamParser::new(buf_reader);

            let mut count = 0;
            while let Ok(Some(_)) = parser.next_event() {
                count += 1;
            }
            black_box(count);
        });
    });

    // Bulk parsing (collect all events)
    group.bench_function("bulk_collect", |b| {
        b.iter(|| {
            let cursor = Cursor::new(msg.as_bytes());
            let buf_reader = BufReader::new(cursor);
            let mut parser = StreamParser::new(buf_reader);

            let mut events = Vec::new();
            while let Ok(Some(event)) = parser.next_event() {
                events.push(event);
            }
            black_box(events);
        });
    });

    group.finish();
}

// =============================================================================
// Buffer Size Benchmarks
// =============================================================================

fn bench_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_sizes");

    let msg = generate_large_message(1000);

    for buffer_size in [64, 256, 1024, 4096, 16384].iter() {
        group.bench_with_input(BenchmarkId::new("buffer", buffer_size), buffer_size, |b, &size| {
            b.iter(|| {
                let cursor = Cursor::new(msg.as_bytes());
                let buf_reader = BufReader::with_capacity(size, cursor);
                let mut parser = StreamParser::new(buf_reader);

                let mut count = 0;
                while let Ok(Some(_)) = parser.next_event() {
                    count += 1;
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Real-World Message Benchmarks
// =============================================================================

fn bench_real_world_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    // ADT^A01 style message
    let adt_a01 = concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r"
    );

    // ORU^R01 style message with multiple OBX
    let mut oru_r01 = String::from(concat!(
        "MSH|^~\\&|LabSys|Lab|LIS|Hospital|",
        "20250128150000||ORU^R01|MSG003|P|2.5\r",
        "PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r",
        "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\r"
    ));
    for i in 1..=50 {
        oru_r01.push_str(&format!(
            "OBX|{}|NM|TEST{}^Test Name||{}.{}|units|low-high|N|||F\r",
            i, i, i, i % 10
        ));
    }

    group.throughput(Throughput::Bytes(adt_a01.len() as u64));
    group.bench_function("adt_a01", |b| {
        b.iter(|| parse_and_count(black_box(adt_a01)));
    });

    group.throughput(Throughput::Bytes(oru_r01.len() as u64));
    group.bench_function("oru_r01_with_50_obx", |b| {
        b.iter(|| parse_and_count(black_box(&oru_r01)));
    });

    group.finish();
}

// =============================================================================
// Event Processing Benchmarks
// =============================================================================

fn bench_event_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_processing");

    let msg = generate_large_message(500);
    let events = parse_and_collect(&msg);

    group.bench_function("count_segments", |b| {
        b.iter(|| {
            let count = events
                .iter()
                .filter(|e| matches!(e, Event::Segment { .. }))
                .count();
            black_box(count);
        });
    });

    group.bench_function("count_fields", |b| {
        b.iter(|| {
            let count = events
                .iter()
                .filter(|e| matches!(e, Event::Field { .. }))
                .count();
            black_box(count);
        });
    });

    group.bench_function("sum_field_lengths", |b| {
        b.iter(|| {
            let total: usize = events
                .iter()
                .filter_map(|e| {
                    if let Event::Field { raw, .. } = e {
                        Some(raw.len())
                    } else {
                        None
                    }
                })
                .sum();
            black_box(total);
        });
    });

    group.finish();
}

// =============================================================================
// Memory Stress Benchmarks
// =============================================================================

fn bench_memory_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_stress");

    // Very long field (1MB)
    let msg_1mb_field = generate_message_with_long_field(1024 * 1024);
    group.throughput(Throughput::Bytes(msg_1mb_field.len() as u64));
    group.bench_function("1mb_field", |b| {
        b.iter(|| parse_and_count(black_box(&msg_1mb_field)));
    });

    // Many small segments
    let msg_many_segments = generate_large_message(10000);
    group.throughput(Throughput::Bytes(msg_many_segments.len() as u64));
    group.bench_function("10000_segments", |b| {
        b.iter(|| parse_and_count(black_box(&msg_many_segments)));
    });

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(
    benches,
    bench_throughput_small_messages,
    bench_throughput_large_messages,
    bench_field_sizes,
    bench_parse_operations,
    bench_incremental_vs_bulk,
    bench_buffer_sizes,
    bench_real_world_messages,
    bench_event_processing,
    bench_memory_stress,
);

criterion_main!(benches);
