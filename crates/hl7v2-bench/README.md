# HL7 v2 Benchmarks

This crate contains comprehensive benchmarks for the hl7v2-rs library using [Criterion.rs](https://bheisler.github.io/criterion.rs/book/).

## Benchmark Categories

### Core Benchmarks (Existing)

| Benchmark | Description |
|-----------|-------------|
| `parsing` | Message parsing performance |
| `mllp` | MLLP framing operations |
| `escape` | Escape/unescape sequence handling |
| `memory` | Memory allocation patterns |

### Extended Benchmarks (New)

| Benchmark | Description |
|-----------|-------------|
| `validation` | Profile and field validation performance |
| `batch` | Batch file processing (FHS/BHS) |
| `json` | JSON serialization/deserialization |
| `network` | MLLP client/server throughput |
| `template` | Template-based message generation |

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmark Suite

```bash
# Parsing benchmarks
cargo bench --bench parsing

# Validation benchmarks
cargo bench --bench validation

# Batch processing benchmarks
cargo bench --bench batch

# JSON serialization benchmarks
cargo bench --bench json

# Network benchmarks
cargo bench --bench network

# Template generation benchmarks
cargo bench --bench template
```

### Run Specific Benchmark Function

```bash
# Run a specific benchmark within a suite
cargo bench --bench parsing -- parse_small_message

# Run benchmarks matching a pattern
cargo bench --bench validation -- data_type
```

### Generate HTML Reports

Criterion automatically generates HTML reports in `target/criterion/`:

```bash
# After running benchmarks, open the report
open target/criterion/report/index.html
```

## Benchmark Details

### validation.rs

Profiles validation performance across different scenarios:

- **Basic validation**: Simple field checks
- **Strict validation**: Comprehensive rule validation
- **Lenient validation**: Minimal critical checks
- **Data type validation**: DT, TM, TS, NM, ST, ID types
- **Throughput testing**: 1, 10, 100, 1000 messages

```bash
cargo bench --bench validation
```

### batch.rs

Batch file parsing and processing:

- **Single batch parsing**: BHS/BTS wrapped messages
- **File batch parsing**: FHS/FTS with nested batches
- **Message counts**: 10, 100, 1000 messages per batch
- **Large messages**: Messages with many segments
- **Memory efficiency**: Iteration and access patterns

```bash
cargo bench --bench batch
```

### json.rs

JSON serialization performance:

- **to_json**: Convert to serde_json::Value
- **to_json_string**: Compact JSON string
- **to_json_string_pretty**: Formatted JSON
- **Size comparison**: Small, medium, large messages
- **Throughput**: 1, 10, 100, 1000 serializations

```bash
cargo bench --bench json
```

### network.rs

MLLP network performance:

- **MLLP wrapping**: Frame encoding
- **Codec performance**: Encoding/decoding
- **Frame size impact**: Small, medium, large frames
- **Concurrent processing**: 1, 2, 4, 8 concurrent operations
- **Round-trip**: Parse → write → wrap pipeline

```bash
cargo bench --bench network
```

### template.rs

Template-based message generation:

- **Simple templates**: Basic message generation
- **Value sources**: Fixed, From, Numeric, UUID, Date, Gaussian
- **Realistic generators**: Names, addresses, phones, SSN, MRN
- **Corpus generation**: Batch message generation
- **Template complexity**: 2 to 12+ segments

```bash
cargo bench --bench template
```

## Statistical Analysis

Criterion provides:

- **Confidence intervals**: 95% confidence level
- **Outlier detection**: Identifies measurement anomalies
- **Regression detection**: Compares against previous runs
- **Memory tracking**: Via optional memory profiling

### Interpreting Results

```
parse_small_message    time:   [1.2345 µs 1.2456 µs 1.2567 µs]
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) low mild
  2 (2.00%) high mild
```

- **time**: Lower bound, mean, upper bound (95% confidence)
- **outliers**: Measurements outside expected range
- **mild/severe**: Degree of deviation

## Memory Profiling

To track memory usage, use the `dhat` or `valgrind` integration:

```bash
# Using valgrind (Linux)
cargo bench --bench memory -- --profile-time=10

# Using heap tracking
MALLOC_CONF="prof:true,prof_prefix:jeprof.out" cargo bench --bench memory
```

## Performance Tips

1. **Close other applications**: Reduce system noise
2. **Run multiple times**: Get stable measurements
3. **Use `--save-baseline`**: Compare against a known good state
4. **Check HTML reports**: Visual analysis often reveals patterns

### Comparing Against Baseline

```bash
# Save baseline
cargo bench -- --save-baseline main

# After changes, compare
cargo bench -- --baseline main
```

## Continuous Integration

Benchmarks can be run in CI with reduced iterations:

```bash
# Quick CI run (fewer samples)
cargo bench -- --sample-size 10
```

## Adding New Benchmarks

1. Create a new file in `benches/` directory
2. Follow the Criterion pattern:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_my_feature(c: &mut Criterion) {
    c.bench_function("my_feature", |b| {
        b.iter(|| {
            // Your benchmark code here
            black_box(result)
        })
    });
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

3. Add to `Cargo.toml`:

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

## Dependencies

- `criterion`: Benchmarking framework with statistical analysis
- `tokio`: Async runtime for network benchmarks
- `futures`: Async utilities
- `serde_json`: JSON serialization
- `rand`: Random number generation for templates
