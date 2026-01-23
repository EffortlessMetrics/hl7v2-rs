//! Benchmarks for HL7 v2 escape sequence handling performance on clean text

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{unescape_text, escape_text, Delims};

/// Create sample clean text for benchmarking (no special characters)
fn create_sample_clean_text() -> String {
    "This is a normal text string without any special characters that would require escaping".to_string()
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
    clean_text_benches,
    bench_unescape_clean_text,
    bench_escape_clean_text
);

criterion_main!(clean_text_benches);
