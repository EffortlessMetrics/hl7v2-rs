//! Benchmarks for HL7 v2 escape sequence handling performance

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use hl7v2_core::{Delims, escape_text, unescape_text};

/// Create sample escaped text for benchmarking
fn create_sample_escaped_text() -> String {
    "This is a test with \\F\\ field separators and \\S\\ component separators".to_string()
}

/// Create sample unescaped text for benchmarking
fn create_sample_unescaped_text() -> String {
    "This is a test with | field separators and ^ component separators".to_string()
}

/// Create sample clean text for benchmarking
fn create_sample_clean_text() -> String {
    "This is a test with no special characters and no delimiters".to_string()
}

/// Benchmark unescaping text
fn bench_unescape_text(c: &mut Criterion) {
    let text = create_sample_escaped_text();
    let delims = Delims::default();

    c.bench_function("unescape_text", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text
fn bench_escape_text(c: &mut Criterion) {
    let text = create_sample_unescaped_text();
    let delims = Delims::default();

    c.bench_function("escape_text", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark unescaping clean text
fn bench_unescape_clean_text(c: &mut Criterion) {
    let text = create_sample_clean_text();
    let delims = Delims::default();

    c.bench_function("unescape_clean_text", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping clean text
fn bench_escape_clean_text(c: &mut Criterion) {
    let text = create_sample_clean_text();
    let delims = Delims::default();

    c.bench_function("escape_clean_text", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

criterion_group!(
    escape_benches,
    bench_unescape_text,
    bench_escape_text,
    bench_unescape_clean_text,
    bench_escape_clean_text
);

criterion_main!(escape_benches);
