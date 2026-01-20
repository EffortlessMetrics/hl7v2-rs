//! Benchmarks for HL7 v2 escape sequence handling performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{unescape_text, escape_text, Delims};

/// Create sample escaped text for benchmarking
fn create_sample_escaped_text() -> String {
    "This is a test with \\F\\ field separators and \\S\\ component separators".to_string()
}

/// Create sample unescaped text for benchmarking
fn create_sample_unescaped_text() -> String {
    "This is a test with | field separators and ^ component separators".to_string()
}

/// Create sample text that doesn't need unescaping
fn create_sample_no_unescape_needed_text() -> String {
    "This is a simple text that does not need unescaping at all".to_string()
}

/// Benchmark unescaping text
fn bench_unescape_text(c: &mut Criterion) {
    let text = create_sample_escaped_text();
    let no_escape = create_sample_no_unescape_needed_text();
    let delims = Delims::default();
    
    let mut group = c.benchmark_group("unescape");

    group.bench_function("unescape_text_needed", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });

    group.bench_function("unescape_text_not_needed", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&no_escape), black_box(&delims));
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark escaping text
fn bench_escape_text(c: &mut Criterion) {
    let text = create_sample_unescaped_text();
    let no_escape = create_sample_no_unescape_needed_text();
    let delims = Delims::default();
    
    let mut group = c.benchmark_group("escape");

    group.bench_function("escape_text_needed", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });

    group.bench_function("escape_text_not_needed", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&no_escape), black_box(&delims));
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    escape_benches,
    bench_unescape_text,
    bench_escape_text
);

criterion_main!(escape_benches);
