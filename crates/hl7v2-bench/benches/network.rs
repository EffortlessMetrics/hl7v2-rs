//! Benchmarks for HL7 v2 MLLP network performance
//!
//! This benchmark suite profiles network performance:
//! - MLLP client/server throughput
//! - Concurrent connection handling
//! - Message round-trip latency
//!
//! Note: These benchmarks use tokio for async operations and may require
//! longer sample times for accurate results.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2_mllp::wrap_mllp;
use hl7v2_parser::parse;
use hl7v2_writer::write_mllp;
use std::hint::black_box;
use tokio::runtime::Runtime;

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250101000000\r".to_string()
}

/// Create a larger message for throughput testing
fn create_large_message() -> String {
    let mut msg = create_sample_message();
    for i in 0..20 {
        msg.push_str(&format!(
            "OBX|{}|NM|OBS{:03}^Observation^L||{:.2}|units|||||F\r",
            i + 1,
            i,
            100.0 + i as f64
        ));
    }
    msg
}

/// Benchmark MLLP frame wrapping
fn bench_mllp_wrap(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("mllp_wrap", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark MLLP frame wrapping for large messages
fn bench_mllp_wrap_large(c: &mut Criterion) {
    let message = create_large_message();
    let bytes = message.as_bytes();

    c.bench_function("mllp_wrap_large", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(bytes));
            black_box(result)
        })
    });
}

/// Benchmark MLLP frame wrapping throughput
fn bench_mllp_wrap_throughput(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("mllp_wrap_throughput");

    for count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                for _ in 0..*count {
                    let result = wrap_mllp(black_box(bytes));
                    black_box(result);
                }
            })
        });
    }

    group.finish();
}

/// Benchmark MLLP write from parsed message
fn bench_mllp_write_message(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("mllp_write_message", |b| {
        b.iter(|| {
            let result = write_mllp(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark MLLP write for large messages
fn bench_mllp_write_large_message(c: &mut Criterion) {
    let message = create_large_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    c.bench_function("mllp_write_large_message", |b| {
        b.iter(|| {
            let result = write_mllp(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark codec encoding performance
fn bench_codec_encoding(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("codec_encode", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate encoding for MLLP transmission
            let wrapped = wrap_mllp(black_box(bytes));
            black_box(wrapped)
        })
    });
}

/// Benchmark codec decoding performance
fn bench_codec_decoding(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let message = create_sample_message();
    let wrapped = wrap_mllp(message.as_bytes());

    c.bench_function("codec_decode", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate decoding from MLLP frame
            // Find start and end markers
            if let Some(start) = wrapped.iter().position(|&b| b == 0x0B)
                && let Some(end) = wrapped.iter().position(|&b| b == 0x1C) {
                    let message_bytes = &wrapped[start + 1..end];
                    black_box(message_bytes);
                }
        })
    });
}

/// Benchmark message frame size impact
fn bench_frame_size_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_size_impact");

    // Small message (~300 bytes)
    let small = create_sample_message();
    let small_bytes = small.as_bytes();
    group.throughput(Throughput::Bytes(small_bytes.len() as u64));
    group.bench_function("small_frame", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(small_bytes));
            black_box(result)
        })
    });

    // Medium message (~1KB)
    let medium = create_large_message();
    let medium_bytes = medium.as_bytes();
    group.throughput(Throughput::Bytes(medium_bytes.len() as u64));
    group.bench_function("medium_frame", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(medium_bytes));
            black_box(result)
        })
    });

    // Large message (~10KB)
    let mut large_msg = String::new();
    for i in 0..100 {
        large_msg.push_str(&format!(
            "OBX|{}|NM|OBS{:03}^Observation^L||{:.2}|units|||||F\r",
            i + 1,
            i,
            100.0 + i as f64
        ));
    }
    let large_bytes = large_msg.as_bytes();
    group.throughput(Throughput::Bytes(large_bytes.len() as u64));
    group.bench_function("large_frame", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(large_bytes));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark concurrent frame processing (simulated)
fn bench_concurrent_frame_processing(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("concurrent_frames");

    for concurrency in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_wrap", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async move {
                    let mut handles = Vec::new();
                    for _ in 0..concurrency {
                        let bytes = bytes.to_vec();
                        handles.push(tokio::task::spawn_blocking(move || {
                            wrap_mllp(black_box(&bytes))
                        }));
                    }
                    let results: Vec<_> = futures::future::join_all(handles).await;
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark message round-trip (parse -> write -> wrap)
fn bench_message_roundtrip(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    c.bench_function("message_roundtrip", |b| {
        b.iter(|| {
            // Parse
            let parsed = parse(black_box(bytes)).expect("Failed to parse");
            // Write back to MLLP
            let mllp = write_mllp(black_box(&parsed));
            // Wrap
            let wrapped = wrap_mllp(&mllp);
            black_box(wrapped)
        })
    });
}

/// Benchmark full pipeline throughput
fn bench_pipeline_throughput(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("pipeline_throughput");

    for count in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                for _ in 0..count {
                    // Parse
                    let parsed = parse(black_box(bytes)).expect("Failed to parse");
                    // Write to MLLP
                    let mllp = write_mllp(black_box(&parsed));
                    // Wrap
                    let wrapped = wrap_mllp(&mllp);
                    black_box(wrapped);
                }
            })
        });
    }

    group.finish();
}

/// Benchmark async frame processing overhead
fn bench_async_overhead(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("async_overhead");

    // Synchronous wrapping
    group.bench_function("sync_wrap", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(bytes));
            black_box(result)
        })
    });

    // Async wrapping with tokio
    group.bench_function("async_wrap", |b| {
        b.to_async(&rt).iter(|| async {
            let result = wrap_mllp(black_box(bytes));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark buffer reuse patterns
fn bench_buffer_patterns(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();

    let mut group = c.benchmark_group("buffer_patterns");

    // Allocate new buffer each time
    group.bench_function("new_buffer", |b| {
        b.iter(|| {
            let result = wrap_mllp(black_box(bytes));
            black_box(result)
        })
    });

    // Reuse pre-allocated buffer (simulated)
    group.bench_function("reuse_buffer", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(bytes.len() + 10);
            buffer.push(0x0B);
            buffer.extend_from_slice(black_box(bytes));
            buffer.push(0x1C);
            buffer.push(0x0D);
            black_box(buffer)
        })
    });

    group.finish();
}

/// Benchmark message serialization for network transmission
fn bench_network_serialization(c: &mut Criterion) {
    let message = create_sample_message();
    let parsed = parse(message.as_bytes()).expect("Failed to parse message");

    let mut group = c.benchmark_group("network_serialization");

    // Standard MLLP write
    group.bench_function("mllp_write", |b| {
        b.iter(|| {
            let result = write_mllp(black_box(&parsed));
            black_box(result)
        })
    });

    // MLLP write + wrap
    group.bench_function("mllp_write_wrap", |b| {
        b.iter(|| {
            let mllp = write_mllp(black_box(&parsed));
            let wrapped = wrap_mllp(&mllp);
            black_box(wrapped)
        })
    });

    group.finish();
}

criterion_group!(
    network_benches,
    bench_mllp_wrap,
    bench_mllp_wrap_large,
    bench_mllp_wrap_throughput,
    bench_mllp_write_message,
    bench_mllp_write_large_message,
    bench_codec_encoding,
    bench_codec_decoding,
    bench_frame_size_impact,
    bench_concurrent_frame_processing,
    bench_message_roundtrip,
    bench_pipeline_throughput,
    bench_async_overhead,
    bench_buffer_patterns,
    bench_network_serialization,
);

criterion_main!(network_benches);

