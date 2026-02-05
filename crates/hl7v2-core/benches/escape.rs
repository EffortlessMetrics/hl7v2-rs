//! Benchmarks for HL7 v2 escape sequence handling performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{unescape_text, escape_text, Delims};

/// Create sample escaped text for benchmarking
fn create_sample_escaped_text() -> String {
    "This is a test with \\F\\ field separators and \\S\\ component separators".to_string()
}

/// Create sample unescaped text for benchmarking (dirty)
fn create_sample_unescaped_dirty_text() -> String {
    "This is a test with | field separators and ^ component separators".to_string()
}

/// Create sample text that needs no escaping/unescaping (clean)
fn create_sample_clean_text() -> String {
    "This is a simple text with no special characters 1234567890".to_string()
}

/// Benchmark unescaping text (dirty)
fn bench_unescape_dirty(c: &mut Criterion) {
    let text = create_sample_escaped_text();
    let delims = Delims::default();
    
    c.bench_function("unescape_dirty", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark unescaping text (clean)
fn bench_unescape_clean(c: &mut Criterion) {
    let text = create_sample_clean_text();
    let delims = Delims::default();

    c.bench_function("unescape_clean", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text (dirty)
fn bench_escape_dirty(c: &mut Criterion) {
    let text = create_sample_unescaped_dirty_text();
    let delims = Delims::default();

    c.bench_function("escape_dirty", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text (clean)
fn bench_escape_clean(c: &mut Criterion) {
    let text = create_sample_clean_text();
    let delims = Delims::default();
    
    c.bench_function("escape_clean", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

criterion_group!(
    escape_benches,
    bench_unescape_dirty,
    bench_unescape_clean,
    bench_escape_dirty,
    bench_escape_clean
);

criterion_main!(escape_benches);
