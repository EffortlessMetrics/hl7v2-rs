# Testing Guide

Comprehensive testing procedures for hl7v2-rs development.

---

## Test Targets for v1.2.0

| Component | Target | Current | Status |
|-----------|--------|---------|--------|
| hl7v2-core | 95% | ~90% | 🟡 In Progress |
| hl7v2-prof | 95% | ~92% | 🟡 In Progress |
| hl7v2-gen | 90% | ~88% | 🟡 In Progress |
| hl7v2-cli | 85% | ~80% | 🟡 In Progress |
| **Overall** | **90%** | **87%** | 🟡 In Progress |

---

## Running Tests

### Quick Start

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p hl7v2-core
cargo test -p hl7v2-prof
cargo test -p hl7v2-gen
cargo test -p hl7v2-cli

# Run specific test
cargo test test_parse_simple
```

### Test Output

```bash
# Show println! output
cargo test -- --nocapture

# Show full output
cargo test -- --nocapture --test-threads=1

# Run ignored tests
cargo test -- --ignored
```

### Test Filtering

```bash
# Run tests matching pattern
cargo test profile           # All tests with "profile" in name
cargo test streaming         # All streaming tests

# Run exact test
cargo test --test test_name -- --exact

# Run single test from command line
cargo test parse_simple::test_simple -- --exact
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
    fn test_parse_simple() {
        let input = b"MSH|^~\\&|...";
        let msg = parse(input).unwrap();
        assert_eq!(msg.segments.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let result = parse(b"");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic]
    fn test_panic_on_invalid() {
        parse_unchecked(b"invalid").unwrap();
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
├── common/
│   └── mod.rs           # Shared utilities
├── parse_integration.rs  # Integrated parse tests
├── validate_integration.rs
└── generate_integration.rs
```

### Example

```rust
// tests/parse_integration.rs

use hl7v2_core::parse;

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
    let serialized = hl7v2_core::write(&msg);

    let msg2 = parse(&serialized).unwrap();
    assert_eq!(msg, msg2);
}
```

---

## Property-Based Testing

Use `proptest` for generating random test cases:

```bash
# Add proptest as dev dependency
cargo add proptest --dev -p hl7v2-core
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
cargo tarpaulin -p hl7v2-core --out Html

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

```bash
# Run in GitHub Actions
- name: Generate coverage
  run: cargo tarpaulin --all --timeout 600 --out Xml

- name: Upload to codecov
  uses: codecov/codecov-action@v3
  with:
    files: ./cobertura.xml
```

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
        b.iter(|| hl7v2_core::parse(input))
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let input = black_box(include_bytes!("../test_data/large.hl7"));

    c.bench_function("parse_large_message", |b| {
        b.iter(|| hl7v2_core::parse(input))
    });
}

criterion_group!(benches, bench_parse_small, bench_parse_large);
criterion_main!(benches);
```

### Performance Targets (v1.2.0)

From [ROADMAP.md](ROADMAP.md):

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Parse (small) | <1ms p95 | 200-byte message |
| Parse (large) | <5ms p95 | 2KB message |
| Validate | <10ms p95 | Typical profile |
| Generate | <2ms p95 | Single message |
| Server throughput | ≥1000 RPS | Sustained load |

### Memory Targets

```bash
# Test RSS memory usage
cargo test --release -- --nocapture --test-threads=1

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
cargo fmt --all && cargo clippy --all && cargo test --all
```

### CI/CD Pipeline

Typical checks (run automatically on PR):

1. **Build**: `cargo build --all`
2. **Format**: `cargo fmt --all -- --check`
3. **Clippy**: `cargo clippy --all -- -D warnings`
4. **Tests**: `cargo test --all`
5. **Coverage**: `cargo tarpaulin --all --fail-under 90`
6. **Benchmarks**: `cargo bench --all`

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.89

      - name: Format
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all -- -D warnings

      - name: Tests
        run: cargo test --all

      - name: Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --all --fail-under 90
```

---

## Troubleshooting Tests

### Test hangs/times out

```bash
# Run with timeout
timeout 30s cargo test test_name

# Run single threaded (helps debug race conditions)
cargo test -- --test-threads=1

# Show backtraces
RUST_BACKTRACE=full cargo test
```

### Test fails locally but passes in CI

```bash
# Try running in release mode
cargo test --release

# Try deterministic ordering
cargo test -- --test-threads=1

# Check for unset environment variables
env | grep HL7
```

### Flaky tests

```rust
// Increase timeout for flaky tests
#[test]
#[ignore]  // Disabled by default
fn test_with_timeout() {
    // test code
}

// Run only with: cargo test -- --ignored
```

---

## Test Data Management

### Test Data Location

```
test_data/
├── valid/
│   ├── oru_r01.hl7
│   ├── adt_a01.hl7
│   └── orm_o01.hl7
├── invalid/
│   ├── malformed.hl7
│   ├── truncated.hl7
│   └── invalid_delim.hl7
└── large/
    └── bulk_10mb.hl7
```

### Include Test Data

```rust
let data = include_bytes!("../../test_data/valid/oru_r01.hl7");
let msg = parse(data).unwrap();
```

### Generating Test Data

```bash
# Using the CLI
cargo run -p hl7v2-cli -- gen --template templates/adt_a01.yaml --count 100 --seed 42 --out test_data/

# Or programmatically in tests
use hl7v2_gen::generate;

let template = load_template("adt_a01.yaml").unwrap();
let msg = generate(&template, 42).unwrap();
```

---

## Definition of Done for Tests

A test contribution is **DONE** when:

- ✅ Tests are added for new functionality
- ✅ Existing tests still pass
- ✅ Coverage is ≥90% for new code
- ✅ Tests run in <10 seconds (unit tests)
- ✅ Tests are deterministic (don't flake)
- ✅ Tests have clear names describing what they test
- ✅ Tests are organized with related tests grouped
- ✅ No test-only code in main library
- ✅ Benchmark baselines established (if applicable)

---

## Quick Reference

```bash
# Essential commands
cargo test --all                          # Run all tests
cargo test -p hl7v2-core                 # Specific crate
cargo test test_name -- --exact           # Specific test
cargo test -- --nocapture                # Show output
cargo bench                                # Run benchmarks
cargo tarpaulin --all --out Html         # Generate coverage
cargo clippy --all                        # Check for issues
cargo fmt --all                           # Format code
```

---

**Remember**: Tests are documentation. Write clear tests that show how to use the code.

For questions, check [DEVELOPMENT.md](DEVELOPMENT.md) or [CONTRIBUTING.md](CONTRIBUTING.md).
