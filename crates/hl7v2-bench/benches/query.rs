//! Benchmarks for HL7 v2 query/path lookup performance.
//!
//! These benchmarks establish the baseline required before changing query
//! indexing or compiled-path behavior.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2::{Message, get, get_presence, parse, parse_located_path};
use std::hint::black_box;

fn create_query_message(obx_count: usize) -> String {
    let mut message = String::from(
        "MSH|^~\\&|BenchApp|BenchFacility|Receiver|ReceiverFacility|20260601010101||ORU^R01^ORU_R01|QRY00001|P|2.5.1\r\
PID|1||123456^^^HOSP^MR~ALT999^^^ALT^MR||Doe^John^A||19800101|M\r\
PV1|1|O|OP^PAREG^CHAREG||||1234^Primary^Provider\r\
OBR|1|ORDER1|FILL1|CBC^Complete blood count\r",
    );

    for index in 1..=obx_count {
        message.push_str(&format!(
            "OBX|{index}|ST|CODE{index:03}^Observation {index}^L||VALUE{index:03}|mg/dL|||||F\r"
        ));
        message.push_str(&format!("NTE|{index}|L|operator note {index}\r"));
    }

    message
}

fn parse_query_message(obx_count: usize) -> Message {
    let message = create_query_message(obx_count);
    match parse(message.as_bytes()) {
        Ok(message) => message,
        Err(err) => {
            eprintln!("query benchmark message failed to parse: {err}");
            std::process::abort();
        }
    }
}

fn query_paths() -> [&'static str; 8] {
    [
        "MSH-10",
        "MSH-12",
        "PID-3[2].4",
        "PID-5.1",
        "PV1-7.2",
        "OBX[1]-5",
        "OBX[30]-5",
        "NTE[30]-3",
    ]
}

fn bench_parse_query_paths(c: &mut Criterion) {
    let paths = query_paths();

    c.bench_function("query_parse_paths_mixed_formats", |b| {
        b.iter(|| {
            for path in paths {
                let parsed = match parse_located_path(black_box(path)) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        eprintln!("query benchmark path failed to parse: {err}");
                        std::process::abort();
                    }
                };
                black_box(parsed);
            }
        });
    });
}

fn bench_repeated_late_segment_get(c: &mut Criterion) {
    let message = parse_query_message(60);

    c.bench_function("query_get_late_repeated_segment", |b| {
        b.iter(|| {
            let value = get(black_box(&message), black_box("OBX[60]-5"));
            black_box(value);
        });
    });
}

fn bench_repeated_late_segment_presence(c: &mut Criterion) {
    let message = parse_query_message(60);

    c.bench_function("query_presence_late_repeated_segment", |b| {
        b.iter(|| {
            let presence = get_presence(black_box(&message), black_box("NTE[60]-3"));
            black_box(presence);
        });
    });
}

fn bench_mixed_query_set(c: &mut Criterion) {
    let message = parse_query_message(60);
    let paths = query_paths();

    c.bench_function("query_get_mixed_path_set", |b| {
        b.iter(|| {
            for path in paths {
                let value = get(black_box(&message), black_box(path));
                black_box(value);
            }
        });
    });
}

fn bench_query_by_segment_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_get_last_obx_by_segment_count");

    for obx_count in [10_usize, 50, 100] {
        let message = parse_query_message(obx_count);
        let path = format!("OBX[{obx_count}]-5");
        group.throughput(Throughput::Elements(obx_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(obx_count),
            &(message, path),
            |b, (message, path)| {
                b.iter(|| {
                    let value = get(black_box(message), black_box(path.as_str()));
                    black_box(value);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    query_benches,
    bench_parse_query_paths,
    bench_repeated_late_segment_get,
    bench_repeated_late_segment_presence,
    bench_mixed_query_set,
    bench_query_by_segment_count,
);

criterion_main!(query_benches);
