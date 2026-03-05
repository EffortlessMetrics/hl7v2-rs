//! Benchmarks for HL7 v2 JSON serialization performance
//!
//! This benchmark suite profiles JSON serialization performance:
//! - Message to JSON serialization
//! - JSON to message deserialization (if supported)
//! - Comparison with other serialization formats

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2_json::{to_json, to_json_string, to_json_string_pretty};
use hl7v2_parser::parse;
use hl7v2_writer::write;
use serde_json::Value;
use std::hint::black_box;

/// Create a sample HL7 message for benchmarking
fn create_sample_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101000000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250101000000\r".to_string()
}

/// Create a complex message with many segments
fn create_complex_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S||123456789||||||||||||||||20250101\rPV1|1|I|ICU^101^01||||DOE^JOHN^A^III^^^^MD|||SUR||||||ADM|12345678|||||||||||||||||||||||||20250101120000\rOBX|1|NM|HEIGHT^Height^L||180|cm|||||F\rOBX|2|NM|WEIGHT^Weight^L||75|kg|||||F\rOBX|3|ST|BP^Blood Pressure^L||120/80|mmHg|||||F\rOBX|4|NM|HR^Heart Rate^L||72|bpm|||||F\rOBX|5|NM|TEMP^Temperature^L||37.0|C|||||F\rAL1|1|DA|PENICILLIN^Penicillin^L||RASH||20200101\rAL1|2|DA|ASPIRIN^Aspirin^L||ANAPHYLAXIS||20190101\rDG1|1|ICD10|J18.9^Pneumonia||20250101||A\rDG1|2|ICD10|E11.9^Type 2 Diabetes||20240101||A\r".to_string()
}

/// Create a message with many repetitions and nested components
fn create_nested_message() -> String {
    "MSH|^~\\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20250101120000||ADT^A01^ADT_A01|MSG00001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John^A^Jr^Dr^MD||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212~(555)555-1213~(555)555-1214||E||S||123456789||||||||||||||||20250101\rNK1|1|Doe^Jane^A^Mrs^MD|SPO|456 Oak St^^Anytown^ST^12345||(555)555-2222||SPO||||||||||||||||||||||||||||||||\rNK1|2|Doe^Bob^A|BRO|789 Pine St^^Anytown^ST^12345||(555)555-3333||BRO||||||||||||||||||||||||||||||||\r".to_string()
}

/// Benchmark JSON serialization to Value
fn bench_to_json_value(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    c.bench_function("to_json_value", |b| {
        b.iter(|| {
            let result = to_json(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark JSON serialization to string
fn bench_to_json_string(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    c.bench_function("to_json_string", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark pretty JSON serialization
fn bench_to_json_string_pretty(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    c.bench_function("to_json_string_pretty", |b| {
        b.iter(|| {
            let result = to_json_string_pretty(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark JSON serialization with complex messages
fn bench_json_complex_message(c: &mut Criterion) {
    let message = create_complex_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    let mut group = c.benchmark_group("json_complex");

    group.bench_function("to_json", |b| {
        b.iter(|| {
            let result = to_json(black_box(&parsed));
            black_box(result)
        })
    });

    group.bench_function("to_json_string", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&parsed));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark JSON serialization with nested structures
fn bench_json_nested_message(c: &mut Criterion) {
    let message = create_nested_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    c.bench_function("json_nested_message", |b| {
        b.iter(|| {
            let result = to_json(black_box(&parsed));
            black_box(result)
        })
    });
}

/// Benchmark JSON serialization throughput
fn bench_json_throughput(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    let mut group = c.benchmark_group("json_throughput");

    for count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::new("serialize", count), count, |b, _| {
            b.iter(|| {
                for _ in 0..*count {
                    let result = to_json(black_box(&parsed));
                    black_box(result);
                }
            })
        });
    }

    group.finish();
}

/// Benchmark JSON string size comparison
fn bench_json_size_comparison(c: &mut Criterion) {
    let simple_msg = create_sample_message();
    let complex_msg = create_complex_message();
    let nested_msg = create_nested_message();

    let simple_parsed = parse(simple_msg.as_bytes()).expect("Failed to parse");
    let complex_parsed = parse(complex_msg.as_bytes()).expect("Failed to parse");
    let nested_parsed = parse(nested_msg.as_bytes()).expect("Failed to parse");

    let mut group = c.benchmark_group("json_size_comparison");

    // Simple message
    let simple_json = to_json_string(&simple_parsed);
    group.throughput(Throughput::Bytes(simple_json.len() as u64));
    group.bench_function("simple_message", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&simple_parsed));
            black_box(result)
        })
    });

    // Complex message
    let complex_json = to_json_string(&complex_parsed);
    group.throughput(Throughput::Bytes(complex_json.len() as u64));
    group.bench_function("complex_message", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&complex_parsed));
            black_box(result)
        })
    });

    // Nested message
    let nested_json = to_json_string(&nested_parsed);
    group.throughput(Throughput::Bytes(nested_json.len() as u64));
    group.bench_function("nested_message", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&nested_parsed));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark JSON vs HL7 wire format comparison
fn bench_json_vs_hl7_format(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    let mut group = c.benchmark_group("json_vs_hl7");

    group.bench_function("json_serialize", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&parsed));
            black_box(result)
        })
    });

    group.bench_function("hl7_serialize", |b| {
        b.iter(|| {
            let result = write(black_box(&parsed));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark JSON value access patterns
fn bench_json_value_access(c: &mut Criterion) {
    let message = create_complex_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");
    let json_value = to_json(&parsed);

    c.bench_function("json_value_access", |b| {
        b.iter(|| {
            // Access various parts of the JSON structure
            if let Value::Object(obj) = &json_value
                && let Some(Value::Array(segments)) = obj.get("segments") {
                    for segment in segments {
                        if let Value::Object(seg) = segment {
                            if let Some(Value::String(id)) = seg.get("id") {
                                black_box(id);
                            }
                            if let Some(Value::Array(fields)) = seg.get("fields") {
                                for field in fields {
                                    black_box(field);
                                }
                            }
                        }
                    }
                }
        })
    });
}

/// Benchmark JSON serialization with different segment counts
fn bench_json_by_segment_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_segment_count");

    // 3 segments (MSH, PID, PV1)
    let small_msg = create_sample_message();
    let small_parsed = parse(small_msg.as_bytes()).expect("Failed to parse");
    group.bench_function("3_segments", |b| {
        b.iter(|| {
            let result = to_json(black_box(&small_parsed));
            black_box(result)
        })
    });

    // 13 segments (MSH, PID, PV1, 5x OBX, 2x AL1, 2x DG1)
    let medium_msg = create_complex_message();
    let medium_parsed = parse(medium_msg.as_bytes()).expect("Failed to parse");
    group.bench_function("13_segments", |b| {
        b.iter(|| {
            let result = to_json(black_box(&medium_parsed));
            black_box(result)
        })
    });

    // Create a larger message with more segments
    let mut large_msg = create_complex_message();
    for i in 0..50 {
        large_msg.push_str(&format!(
            "OBX|{}|ST|OBS{:03}^Observation^L||Value|units|||||F\r",
            6 + i,
            i
        ));
    }
    let large_parsed = parse(large_msg.as_bytes()).expect("Failed to parse");
    group.bench_function("63_segments", |b| {
        b.iter(|| {
            let result = to_json(black_box(&large_parsed));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark memory allocation during JSON serialization
fn bench_json_memory_allocation(c: &mut Criterion) {
    let message = create_sample_message();
    let bytes = message.as_bytes();
    let parsed = parse(bytes).expect("Failed to parse message");

    let mut group = c.benchmark_group("json_memory");

    // Reuse JSON value (to String)
    group.bench_function("to_string_reuse", |b| {
        b.iter(|| {
            let json_value = to_json(black_box(&parsed));
            let string = serde_json::to_string(&json_value).unwrap();
            black_box(string)
        })
    });

    // Direct to string
    group.bench_function("to_string_direct", |b| {
        b.iter(|| {
            let result = to_json_string(black_box(&parsed));
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    json_benches,
    bench_to_json_value,
    bench_to_json_string,
    bench_to_json_string_pretty,
    bench_json_complex_message,
    bench_json_nested_message,
    bench_json_throughput,
    bench_json_size_comparison,
    bench_json_vs_hl7_format,
    bench_json_value_access,
    bench_json_by_segment_count,
    bench_json_memory_allocation,
);

criterion_main!(json_benches);

