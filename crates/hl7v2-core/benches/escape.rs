//! Benchmarks for HL7 v2 escape sequence handling performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hl7v2_core::{unescape_text, escape_text, Delims};

/// Create clean text (no escaping needed) for benchmarking
fn create_clean_text() -> String {
    "This is a clean text with no special characters and only normal ascii that does not need any escaping or unescaping".to_string()
}

/// Create dirty text (needs escaping) for benchmarking
fn create_dirty_text() -> String {
    "This is a dirty text with | field separators and ^ component separators and ~ repetition separators and \\ escape chars and & subcomponent separators".to_string()
}

/// Create dirty escaped text (needs unescaping) for benchmarking
fn create_dirty_escaped_text() -> String {
    "This is a dirty text with \\F\\ field separators and \\S\\ component separators and \\R\\ repetition separators and \\E\\ escape chars and \\T\\ subcomponent separators".to_string()
}

/// Benchmark escaping clean text
fn bench_escape_clean(c: &mut Criterion) {
    let text = create_clean_text();
    let delims = Delims::default();

    c.bench_function("escape_clean", |b| {
        b.iter(|| {
            escape_text(black_box(&text), black_box(&delims))
        })
    });
}

/// Benchmark escaping dirty text
fn bench_escape_dirty(c: &mut Criterion) {
    let text = create_dirty_text();
    let delims = Delims::default();

    c.bench_function("escape_dirty", |b| {
        b.iter(|| {
            escape_text(black_box(&text), black_box(&delims))
        })
    });
}

/// Benchmark unescaping clean text
fn bench_unescape_clean(c: &mut Criterion) {
    let text = create_clean_text();
    let delims = Delims::default();
    
    c.bench_function("unescape_clean", |b| {
        b.iter(|| {
            unescape_text(black_box(&text), black_box(&delims))
        })
    });
}

/// Benchmark unescaping dirty text
fn bench_unescape_dirty(c: &mut Criterion) {
    let text = create_dirty_escaped_text();
    let delims = Delims::default();
    
    c.bench_function("unescape_dirty", |b| {
        b.iter(|| {
            unescape_text(black_box(&text), black_box(&delims))
        })
    });
}

criterion_group!(
    escape_benches,
    bench_escape_clean,
    bench_escape_dirty,
    bench_unescape_clean,
    bench_unescape_dirty
);

criterion_main!(escape_benches);
