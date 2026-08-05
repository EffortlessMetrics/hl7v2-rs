//! Benchmarks for HTTP metrics label recording.
//!
//! The common-status benchmark exercises the middleware's static status-label
//! path. The custom-status benchmark documents the owned fallback retained for
//! caller-provided or otherwise uncommon status values.

use criterion::{Criterion, criterion_group, criterion_main};
use hl7v2_server::metrics::{init_metrics_recorder, record_request};
use std::hint::black_box;

fn bench_request_labels(c: &mut Criterion) {
    let _handle = init_metrics_recorder();
    let mut group = c.benchmark_group("request_labels");

    group.bench_function("common_status", |b| {
        b.iter(|| record_request(black_box("/hl7/parse"), black_box("200"), 0.001));
    });

    group.bench_function("custom_status", |b| {
        b.iter(|| record_request(black_box("/hl7/parse"), black_box("599"), 0.001));
    });

    group.finish();
}

criterion_group!(metrics_benches, bench_request_labels);
criterion_main!(metrics_benches);
