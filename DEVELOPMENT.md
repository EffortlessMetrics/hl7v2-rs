# Development Guide

Get up and running with hl7v2-rs development in 10 minutes.

---

## Prerequisites

- **Rust**: 1.92+ (MSRV)
  - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Update: `rustup update`

- **Git**: For cloning and committing

- **Optional but Recommended**:
  - VS Code with rust-analyzer
  - Cargo-watch for auto-rebuilds
  - cargo-tarpaulin for coverage

---

## Project Setup

### 1. Clone the Repository

```bash
git clone https://github.com/EffortlessMetrics/hl7v2-rs.git
cd hl7v2-rs
```

### 2. Build the Project

```bash
# Build in debug mode (fast build, slower binary)
cargo build

# Build in release mode (slower build, optimized binary)
cargo build --release

# Build specific crate
cargo build -p hl7v2-core
cargo build -p hl7v2-cli
```

### 3. Verify Installation

```bash
# Run CLI
cargo run --bin hl7v2-cli -- --help

# Or run all tests
cargo test
```

**Expected output**: Help message or test results

---

## Common Development Tasks

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p hl7v2-core
cargo test -p hl7v2-prof
cargo test -p hl7v2-gen

# Run specific test
cargo test test_parse_simple

# Run tests with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### Code Quality Checks

```bash
# Format code (required before commit)
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check

# Lint code
cargo clippy --all

# Clippy with all lints
cargo clippy --all -- -W clippy::all

# Audit dependencies for vulnerabilities
cargo audit
```

**Pre-commit checklist**:
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all` has no warnings
- [ ] `cargo test --all` passes
- [ ] No new clippy warnings

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --all

# Run specific benchmark
cargo bench -- --exact parsing_small_message

# Compare against baseline
cargo bench -- --baseline main_branch

# Save baseline for future comparison
cargo bench -- --save-baseline my_feature
```

**Benchmark locations**:
- `crates/hl7v2-core/benches/` - Core parsing/writing
- `crates/hl7v2-gen/benches/` - Generation benchmarks
- `crates/hl7v2-prof/benches/` - Validation benchmarks

### Measuring Code Coverage

```bash
# Install tarpaulin (first time only)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --all --out Html

# Open report
open tarpaulin-report.html  # macOS
xdg-open tarpaulin-report.html  # Linux
start tarpaulin-report.html  # Windows
```

**Coverage targets**:
- Core functionality: 95%+
- New code: 90%+
- Overall: 85%+

### Watching for Changes

```bash
# Install cargo-watch (first time only)
cargo install cargo-watch

# Auto-test on file changes
cargo watch -x test

# Auto-build on changes
cargo watch -x build

# Auto-check code (fast)
cargo watch -x check
```

---

## IDE/Editor Setup

### VS Code (Recommended)

**Extensions**:
- rust-analyzer
- Rust (rdbg)
- CodeLLDB (for debugging)

**.vscode/settings.json**:
```json
{
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### IntelliJ IDEA

- Install Rust plugin
- Enable code inspections
- Configure run configurations

### vim/neovim

Use `coc-rust-analyzer` for LSP support

---

## Project Structure

The project is organized as a Cargo workspace with 28 specialized crates in the `crates/` directory, categorized into three layers:

### 1. Microcrates (SRP-focused)
Minimal dependencies, single-responsibility logic.
- `hl7v2-model`, `hl7v2-parser`, `hl7v2-writer`, `hl7v2-escape`, `hl7v2-json`, `hl7v2-normalize`, `hl7v2-datetime`, `hl7v2-datatype`, `hl7v2-path`, `hl7v2-query`, `hl7v2-batch`.

### 2. Feature & Service Crates
- `hl7v2-network` (MLLP over TCP/TLS), `hl7v2-stream` (Event-based parsing), `hl7v2-validation` (Rule engine), `hl7v2-prof` (Conformance profiles), `hl7v2-ack`, `hl7v2-faker`, `hl7v2-template`.

### 3. Application & High-level Crates
- `hl7v2-core` (Facade), `hl7v2-cli`, `hl7v2-server` (Axum REST API), `hl7v2-bench`.

### Key Files

- `Cargo.toml` - Workspace dependencies and versions
- `crates/*/Cargo.toml` - Individual crate configuration
- `crates/hl7v2-core/src/lib.rs` - Core public API facade
- `crates/hl7v2-cli/src/main.rs` - CLI entry point

---

## Running the CLI Locally

### Build CLI

```bash
cargo build -p hl7v2-cli
```

### Run Commands

```bash
# Using cargo run
cargo run -p hl7v2-cli -- parse test_data/sample.hl7

# Or install and run directly
cargo install --path crates/hl7v2-cli
hl7v2 parse test_data/sample.hl7
```

### Create Test Data

```bash
# Generate test message
echo "MSH|^~\\&|SENDER|FACILITY|RECEIVER|FACILITY|20230101120000||ADT^A01|MSG123|P|2.5
PID|1||123456^^^MRN||Doe^John||19800101|M" > test.hl7

# Test parsing
cargo run -p hl7v2-cli -- parse test.hl7
```

---

## Debugging

### Using Rust Debugger (LLDB/GDB)

**VS Code with CodeLLDB**:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug test",
      "cargo": {
        "args": [
          "test",
          "--package",
          "hl7v2-core"
        ],
        "filter": {
          "name": "test_parse_simple",
          "kind": "test"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Logging/Printing

```rust
// Simple debug printing
dbg!(variable);

// Or use println
println!("Debug: {:?}", variable);

// In tests
cargo test -- --nocapture  // Show println! output
```

### Using cargo expand

```bash
# Install (first time only)
cargo install cargo-expand

# Expand macros in a file
cargo expand crates/hl7v2-core::src::lib
```

---

## Dependency Management

### Check for Vulnerabilities

```bash
cargo audit
```

### Update Dependencies

```bash
# Check for outdated deps
cargo outdated

# Update patch versions
cargo update

# Update specific crate
cargo update -p serde
```

### Add New Dependency

```bash
# Production dependency
cargo add serde --package hl7v2-core

# Development/test dependency
cargo add proptest --dev --package hl7v2-core
```

---

## Working with Specific Crates

### Focus on Core Changes

```bash
# Build only core
cargo build -p hl7v2-core

# Test only core
cargo test -p hl7v2-core

# Clippy only core
cargo clippy -p hl7v2-core

# Watch for changes in core
cargo watch -p hl7v2-core -x test
```

### Profile Validation Development

```bash
# Build profile crate
cargo build -p hl7v2-prof

# Run profile tests
cargo test -p hl7v2-prof

# Watch for profile changes
cargo watch -p hl7v2-prof -x test
```

---

## Testing Tips

### Running a Single Test

```bash
# Run exact test
cargo test test_profile_inheritance -- --exact

# Run tests matching pattern
cargo test profile  # runs all tests with "profile" in name

# Run with backtrace
RUST_BACKTRACE=1 cargo test
```

### Test Organization

Tests can live in:
1. `src/lib.rs` (unit tests with `#[test]`)
2. `src/tests.rs` (unit test module)
3. `tests/` directory (integration tests)

**Best practice**: Put unit tests in `tests` module of same file:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // test code
    }
}
```

---

## Common Issues & Solutions

### Issue: Tests fail with "file not found"

**Solution**: Run tests from workspace root:
```bash
cd /path/to/hl7v2-rs  # Not from crates/ subdirectory
cargo test
```

### Issue: Clippy complains about something I disagree with

**Solution**: You can suppress specific lints:
```rust
#[allow(clippy::some_lint)]
fn my_function() { }
```

Document the reason in a comment above.

### Issue: Compilation is slow

**Solution**: Use `cargo check` instead of `cargo build`:
```bash
# Much faster (doesn't generate binary)
cargo check

# Watch for changes
cargo watch -x check
```

### Issue: "MSRV violated" error

**Solution**: You're using a feature from newer Rust:
```bash
# Check your Rust version
rustc --version

# Update Rust (requires 1.89+)
rustup update
```

---

## Git Workflow

### Before Committing

```bash
# Format code
cargo fmt --all

# Check for issues
cargo clippy --all

# Run tests
cargo test --all
```

### Commit Message Format

See [CONTRIBUTING.md](CONTRIBUTING.md#commit-your-work)

```
feat(core): add bounded queue for backpressure

Implementation details...

Closes #42
```

### Create Feature Branch

```bash
# Create and switch to new branch
git checkout -b feature/my-feature

# Make changes, test, commit
# Push to GitHub
git push -u origin feature/my-feature
```

---

## Performance Profiling

### Linux/macOS with perf

```bash
# Install perf-tools
sudo apt install linux-tools-generic  # Ubuntu
brew install flamegraph  # macOS

# Run with perf
cargo bench -- --profile-time 5

# Generate flame graph
cargo install flamegraph
cargo flamegraph
```

### Memory Profiling

```bash
# Valgrind (Linux)
cargo install cargo-valgrind
cargo valgrind test

# Heaptrack (Linux)
heaptrack cargo test
```

---

## Documentation

### Generate and View Docs

```bash
# Generate documentation
cargo doc --all --no-deps

# Generate and open in browser
cargo doc --all --no-deps --open
```

### Writing Rustdoc

```rust
/// Parses an HL7 v2 message from bytes.
///
/// # Arguments
/// * `input` - Raw HL7 message bytes
///
/// # Returns
/// * `Ok(Message)` - Parsed message
/// * `Err(ParseError)` - Parse failure with details
///
/// # Examples
/// ```
/// use hl7v2_core::parse;
/// let msg = parse(b"MSH|^~\\&|...")?;
/// assert_eq!(msg.segments.len(), 2);
/// ```
pub fn parse(input: &[u8]) -> Result<Message, ParseError> {
    // implementation
}
```

---

## Next Steps

1. **Read CONTRIBUTING.md** for development guidelines
2. **Check IMPLEMENTATION_STATUS.md** for what's being worked on
3. **Look at IMPLEMENTATION_PLAN.md** for v1.2.0 sprint work
4. **Pick an issue** and start coding!

---

## Quick Reference

```bash
# Build & test
cargo build
cargo test --all
cargo fmt --all
cargo clippy --all

# Development workflow
cargo watch -x test              # Auto-test on changes
cargo run -- <args>             # Run CLI
cargo bench                      # Run benchmarks
cargo doc --open                # View docs

# Specific crate
cargo test -p hl7v2-core
cargo clippy -p hl7v2-prof
cargo watch -p hl7v2-cli -x test

# Coverage & profiling
cargo tarpaulin --all --out Html
cargo flamegraph
```

---

## Questions?

- Check the [FAQ in CONTRIBUTING.md](CONTRIBUTING.md#getting-help)
- Open a GitHub discussion
- Ask in team channel (if you have access)

---

**Happy coding!** 🦀
