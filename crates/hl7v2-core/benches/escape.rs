//! Benchmarks for HL7 v2 escape sequence handling performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{unescape_text, unescape_text_cow, escape_text, escape_text_cow, Delims};

/// Create sample escaped text for benchmarking
fn create_sample_escaped_text() -> String {
    "This is a test with \\F\\ field separators and \\S\\ component separators".to_string()
}

/// Create sample unescaped text for benchmarking
fn create_sample_unescaped_text() -> String {
    "This is a test with | field separators and ^ component separators".to_string()
}

/// Create sample text that needs no escaping/unescaping
fn create_noop_text() -> String {
    "This is a simple text with no special characters".to_string()
}

/// Benchmark unescaping text (with escapes) - Standard (allocates)
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

/// Benchmark unescaping text (with escapes) - Cow (allocates)
fn bench_unescape_text_cow(c: &mut Criterion) {
    let text = create_sample_escaped_text();
    let delims = Delims::default();

    c.bench_function("unescape_text_cow", |b| {
        b.iter(|| {
            let result = unescape_text_cow(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark unescaping text (no escapes) - Standard (allocates)
fn bench_unescape_text_noop(c: &mut Criterion) {
    let text = create_noop_text();
    let delims = Delims::default();

    c.bench_function("unescape_text_noop", |b| {
        b.iter(|| {
            let result = unescape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark unescaping text (no escapes) - Cow (zero allocation)
fn bench_unescape_text_noop_cow(c: &mut Criterion) {
    let text = create_noop_text();
    let delims = Delims::default();

    c.bench_function("unescape_text_noop_cow", |b| {
        b.iter(|| {
            let result = unescape_text_cow(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text (with specials) - Standard (allocates)
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

/// Benchmark escaping text (with specials) - Cow (allocates)
fn bench_escape_text_cow(c: &mut Criterion) {
    let text = create_sample_unescaped_text();
    let delims = Delims::default();

    c.bench_function("escape_text_cow", |b| {
        b.iter(|| {
            let result = escape_text_cow(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text (no specials) - Standard (allocates)
fn bench_escape_text_noop(c: &mut Criterion) {
    let text = create_noop_text();
    let delims = Delims::default();

    c.bench_function("escape_text_noop", |b| {
        b.iter(|| {
            let result = escape_text(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

/// Benchmark escaping text (no specials) - Cow (zero allocation)
fn bench_escape_text_noop_cow(c: &mut Criterion) {
    let text = create_noop_text();
    let delims = Delims::default();

    c.bench_function("escape_text_noop_cow", |b| {
        b.iter(|| {
            let result = escape_text_cow(black_box(&text), black_box(&delims));
            black_box(result)
        })
    });
}

criterion_group!(
    escape_benches,
    bench_unescape_text,
    bench_unescape_text_cow,
    bench_unescape_text_noop,
    bench_unescape_text_noop_cow,
    bench_escape_text,
    bench_escape_text_cow,
    bench_escape_text_noop,
    bench_escape_text_noop_cow
);

criterion_main!(escape_benches);
