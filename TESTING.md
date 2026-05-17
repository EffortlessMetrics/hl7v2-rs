# Testing Guide

Comprehensive testing procedures for hl7v2-rs development.

---

## Test Targets for v1.5.0

The current workspace is Rust 1.95 and version `1.5.0`. The public Rust
product graph is `hl7v2`, `hl7v2-cli`, and `hl7v2-server`; `hl7v2-python` is
a binding backend for the public Python `hl7v2` package. Historical crates such
as `hl7v2-core`, `hl7v2-prof`, and `hl7v2-gen` are no longer the current
workspace surface.

| Surface | Target | Current evidence |
|-----------|--------|------------------|
| Rust library (`hl7v2`) | Parser, validation, normalization, ACK, profile, corpus, bundle, replay, and user-journey tests pass on Rust 1.95. | `cargo +1.95.0 test -p hl7v2 --all-features` |
| CLI (`hl7v2-cli`) | Evidence command and integration tests pass. | `cargo +1.95.0 test -p hl7v2-cli --all-features` |
| Server (`hl7v2-server`) | REST/gRPC contracts, health, bundle, replay, quarantine, and evidence endpoint tests pass. | `cargo +1.95.0 test -p hl7v2-server --all-features` |
| Python binding backend (`hl7v2-python`) | Local wheel build/install/import and evidence smoke pass; public TestPyPI/PyPI proof remains separate. | `python tests/python_smoke/smoke.py` and `python tests/python_smoke/evidence_workflow_guide.py` after a local wheel install |
| Policy and release gates | Lint, no-panic, file-policy, evidence-schema, publish-plan, and docs checks remain green. | `cargo +1.95.0 run -p xtask -- gate --check` plus the focused `xtask` checks below |

---

## Running Tests

### Quick Start

```bash
# Run all tests
cargo +1.95.0 test --workspace --all-features

# Run tests for specific crate
cargo +1.95.0 test -p hl7v2 --all-features
cargo +1.95.0 test -p hl7v2-cli --all-features
cargo +1.95.0 test -p hl7v2-server --all-features
cargo +1.95.0 test -p xtask

# Run specific test
cargo +1.95.0 test parse_simple -- --exact
```

### Test Output

```bash
# Show println! output
cargo +1.95.0 test -- --nocapture

# Show full output
cargo +1.95.0 test -- --nocapture --test-threads=1

# Run ignored tests
cargo +1.95.0 test -- --ignored
```

### Test Filtering

```bash
# Run tests matching pattern
cargo +1.95.0 test profile           # All tests with "profile" in name
cargo +1.95.0 test streaming         # All streaming tests

# Run exact test
cargo +1.95.0 test --test test_name -- --exact

# Run single test from command line
cargo +1.95.0 test parse_simple::test_simple -- --exact
```

---

## Unit Tests

### Location

- Add `#[cfg(test)] mod tests { }` to same file as code
- Or put in `src/tests.rs` file
- Or in `tests/` directory for integration tests

### Example Structure

```rust
// src/lib.rs

pub fn parse(input: &[u8]) -> Result<Message> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() -> Result<(), Box<dyn std::error::Error>> {
        let input = b"MSH|^~\\&|...";
        let msg = parse(input)?;
        assert_eq!(msg.segments.len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_empty() {
        let result = parse(b"");
        assert!(result.is_err());
    }

    #[test]
    #[ignore]
    fn test_slow_operation() {
        // Only runs with `cargo test -- --ignored`
    }
}
```

### Assertion Patterns

```rust
// Basic assertions
assert!(condition, "message");
assert_eq!(actual, expected, "message");
assert_ne!(actual, unexpected, "message");

// Option assertions
assert!(option.is_some());
assert!(option.is_none());
assert_eq!(option, Some(value));

// Result assertions
assert!(result.is_ok());
assert!(result.is_err());
assert_eq!(result, Ok(value));
assert_eq!(result, Err(error));

// String assertions
assert!(string.contains("substring"));
assert!(string.starts_with("prefix"));
```

---

## Integration Tests

### Location

Tests in `tests/` directory are compiled as separate binaries:
```
tests/
  common/
    mod.rs                  # Shared utilities
  parse_integration.rs      # Integrated parse tests
  validate_integration.rs
  generate_integration.rs
```

### Example

```rust
// tests/parse_integration.rs

use hl7v2::parse;

#[test]
fn test_full_parse_workflow() {
    let input = include_bytes!("../test_data/sample.hl7");
    let msg = parse(input).unwrap();

    assert_eq!(msg.segments.len(), 5);
    assert_eq!(msg.delims.field, b'|');

    // Verify parsed content
    let pid = &msg.segments[1];
    assert_eq!(pid.id, [b'P', b'I', b'D']);
}

#[test]
fn test_parse_validates_round_trip() {
    let original = include_bytes!("../test_data/sample.hl7");
    let msg = parse(original).unwrap();
    let serialized = hl7v2::write(&msg);

    let msg2 = parse(&serialized).unwrap();
    assert_eq!(msg, msg2);
}
```

---

## Property-Based Testing

Use `proptest` for generating random test cases:

```bash
# Add proptest as dev dependency
cargo add proptest --dev -p hl7v2
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_parse_doesnt_panic(input in ".*") {
        // Parse should never panic on arbitrary input
        let _ = parse(input.as_bytes());
    }

    #[test]
    fn prop_round_trip(delim in '[|^~\\\\&]') {
        let msg = Message { delims: Delims::default(), .. };
        let serialized = write(&msg);
        let parsed = parse(&serialized);

        prop_assert!(parsed.is_ok());
    }
}
```

---

## Coverage Measurement

### Using tarpaulin

```bash
# Install (first time only)
cargo install cargo-tarpaulin

# Generate HTML coverage report
cargo tarpaulin --all --out Html

# Generate with specific output format
cargo tarpaulin --all --out Lcov --output-dir target/coverage
cargo tarpaulin --all --out Xml

# Coverage for specific crate
cargo tarpaulin -p hl7v2 --out Html

# With minimum coverage threshold
cargo tarpaulin --all --timeout 600 --fail-under 90
```

### Using llvm-cov

```bash
# Install
cargo install cargo-llvm-cov

# Generate coverage
cargo llvm-cov --all --html

# Show in terminal
cargo llvm-cov --all
```

### CI/CD Coverage

Coverage is routed rather than part of every default PR lane. See
[docs/ci/test-evidence-lanes.md](docs/ci/test-evidence-lanes.md) and
[docs/ci/coverage.md](docs/ci/coverage.md) for the current
coverage workflow and claim boundaries.

---

## Performance Testing & Benchmarking

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench --all

# Run specific benchmark
cargo bench -- parsing_small

# Run with unstable output format
cargo bench --bench parsing -- --verbose

# Create baseline for comparison
cargo bench -- --save-baseline before_optimization

# Compare against baseline
cargo bench -- --baseline before_optimization
```

### Writing Benchmarks

Location: `crates/*/benches/*.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_small(c: &mut Criterion) {
    let input = black_box(include_bytes!("../test_data/small.hl7"));

    c.bench_function("parse_small_message", |b| {
        b.iter(|| hl7v2::parse(input))
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let input = black_box(include_bytes!("../test_data/large.hl7"));

    c.bench_function("parse_large_message", |b| {
        b.iter(|| hl7v2::parse(input))
    });
}

criterion_group!(benches, bench_parse_small, bench_parse_large);
criterion_main!(benches);
```

### Performance Targets

The benchmark harness lives in `crates/hl7v2-bench`. Treat the table below as
target guidance, not a release receipt. Release claims should point to
benchmark or readiness receipts under `docs/audits/`.

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Parse (small) | <1ms p95 | 200-byte message |
| Parse (large) | <5ms p95 | 2KB message |
| Validate | <10ms p95 | Typical profile |
| ACK/write | <2ms p95 | Single message |
| Server throughput | >=1000 RPS | Sustained load |

### Memory Targets

```bash
# Test RSS memory usage
cargo +1.95.0 test --release -- --nocapture --test-threads=1

# Expected: Proportional to message size, <500MB steady-state
```

---

## Security Testing

### Input Validation

Test with malformed/malicious inputs:

```rust
#[test]
fn test_reject_oversized_message() {
    let huge = vec![b'A'; 100 * 1024 * 1024];  // 100MB
    let result = parse(&huge);
    assert!(result.is_err());
}

#[test]
fn test_reject_invalid_utf8() {
    let invalid = vec![0xFF, 0xFE, 0xFF];
    let result = parse(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_handle_null_bytes() {
    let with_nulls = b"MSH|^~\\&|SENDER\x00INVALID";
    let result = parse(with_nulls);
    assert!(result.is_err());
}
```

### Dependency Vulnerabilities

```bash
# Check for known vulnerabilities
cargo audit

# Fix vulnerabilities
cargo update

# Require zero critical issues in CI
cargo audit --deny warnings
```

---

## Test Organization Best Practices

### Group Related Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod parsing {
        use super::*;

        #[test]
        fn simple_message() { /* ... */ }

        #[test]
        fn complex_message() { /* ... */ }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn invalid_delimiter() { /* ... */ }

        #[test]
        fn missing_segment() { /* ... */ }
    }
}
```

### Test Fixtures

```rust
fn sample_message() -> &'static [u8] {
    b"MSH|^~\\&|SENDER|FAC|RECEIVER|FAC|20230101||ADT^A01|123|P|2.5"
}

fn sample_profile() -> Profile {
    Profile {
        message_structure: "ADT^A01".to_string(),
        version: "2.5".to_string(),
        // ...
    }
}

#[test]
fn test_with_fixtures() {
    let msg = parse(sample_message()).unwrap();
    let profile = sample_profile();

    let issues = validate(&msg, &profile);
    assert!(issues.is_empty());
}
```

### Shared Test Utilities

```rust
// tests/common/mod.rs
pub fn create_test_profile() -> Profile {
    // ...
}

pub fn create_sample_message() -> Message {
    // ...
}

// tests/integration_test.rs
mod common;

#[test]
fn test_something() {
    let profile = common::create_test_profile();
    // ...
}
```

---

## Continuous Integration

### Pre-commit Checks

```bash
# Before committing
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.95.0 test --workspace --all-features --locked
```

### CI/CD Pipeline

Current PR checks are staged and routed:

1. **Fast Checks**: formatting, Clippy, lint policy, no-panic-family, file
   policy, unit tests, and doc tests.
2. **MSRV Smoke (1.95)**: verifies the declared Rust 1.95 support boundary.
3. **Standard Tests**: integration, BDD, and limited property tests.
4. **Matrix / Extended / Benchmarks**: routed by branch, label, or manual
   dispatch so default PR cost stays bounded.
5. **Security / CI Policy / PR Plan**: security, policy, and lane-routing
   checks.

Use the workflow files under `.github/workflows/` as the source of truth for
the current CI implementation. Do not copy old sample workflows into the repo.

---

## Troubleshooting Tests

### Test hangs/times out

```bash
# Run with timeout
cargo +1.95.0 test test_name

# Run single threaded (helps debug race conditions)
cargo +1.95.0 test -- --test-threads=1

# Show backtraces
RUST_BACKTRACE=full cargo +1.95.0 test
```

### Test fails locally but passes in CI

```powershell
# Try running in release mode
cargo +1.95.0 test --release

# Try deterministic ordering
cargo +1.95.0 test -- --test-threads=1

# Check for unset environment variables on Windows PowerShell
Get-ChildItem Env:HL7*
```

### Flaky tests

```rust
// Increase timeout for flaky tests
#[test]
#[ignore]  // Disabled by default
fn test_with_timeout() {
    // test code
}

// Run only with: cargo +1.95.0 test -- --ignored
```

---

## Test Data Management

### Test Data Location

```
test_data/
  valid_message.hl7
  invalid_message.hl7
  test_profile.yaml
  dirty-real-world/
    before/
    after/
    sources/
```

### Include Test Data

```rust
let data = include_bytes!("../../test_data/valid/oru_r01.hl7");
let msg = parse(data).unwrap();
```

### Generating Test Data

```bash
# Using the CLI
cargo +1.95.0 run -p hl7v2-cli -- gen --template test_data/test_template.yaml --count 10 --seed 42 --out target/generated-hl7

# Or programmatically in tests
use hl7v2::synthetic::template::{generate, Template};
use std::collections::HashMap;

let template = Template {
    name: "ADT_A01".to_string(),
    delims: "^~\\&".to_string(),
    segments: vec![
        "MSH|^~\\&|SEND|FAC|RECV|FAC|202605170101||ADT^A01|CTRL1|P|2.5".to_string(),
        "PID|1||MRN-1^^^HOSP^MR||Example^Patient".to_string(),
    ],
    values: HashMap::new(),
};
let messages = generate(&template, 42, 10).unwrap();
```

---

## Definition of Done for Tests

A test contribution is **DONE** when:

- Tests are added for new or changed behavior.
- Existing focused tests still pass.
- Evidence-producing behavior has schema or fixture coverage when applicable.
- Tests are deterministic and do not require raw PHI.
- Test names describe the behavior being protected.
- Test data is safe to commit and documented when non-obvious.
- Policy gates remain green: lint policy, no-panic-family, file policy, and docs links.
- Benchmark or performance receipts are updated when the change affects a measured path.

---

## Quick Reference

```bash
# Essential commands
cargo +1.95.0 test --workspace --all-features --locked
cargo +1.95.0 test -p hl7v2 --all-features
cargo +1.95.0 test test_name -- --exact
cargo +1.95.0 test -- --nocapture
cargo +1.95.0 bench --workspace
cargo +1.95.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 run -p xtask -- gate --check
cargo +1.95.0 run -p xtask -- check-doc-links
```

---

**Remember**: Tests are documentation. Write clear tests that show how to use the code.

For questions, check [DEVELOPMENT.md](DEVELOPMENT.md) or [CONTRIBUTING.md](CONTRIBUTING.md).
