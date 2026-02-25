# CI/CD Pipeline Documentation

This document describes the Continuous Integration and Continuous Deployment (CI/CD) pipeline for the HL7 v2 Rust workspace.

## Pipeline Overview

The CI/CD pipeline follows a4-stage design as outlined in [`TESTING_ARCHITECTURE.md`](./TESTING_ARCHITECTURE.md):

| Stage | Duration | Trigger | Purpose |
|-------|----------|---------|---------|
| Fast | ~2 min | Every PR/push | Quick feedback on code quality |
| Standard | ~5 min | Every PR/push | Integration and BDD tests |
| Extended | ~10 min | Main branch only | Full property tests, coverage, benchmarks |
| Nightly | ~1 hour | Scheduled nightly | Fuzz tests, mutation tests |

## Workflow Files

### `.github/workflows/ci.yml` - Main CI Pipeline

The primary CI workflow that runs on every push and pull request.

#### Jobs:

1. **fast** - Fast feedback checks
   - Format check (`cargo fmt --check`)
   - Clippy lints (`cargo clippy`)
   - Unit tests (`cargo test --lib`)
   - Doc tests (`cargo test --doc`)

2. **standard** - Standard test suite
   - Integration tests (`cargo test --test '*'`)
   - BDD tests (Cucumber tests)
   - Limited property tests (100 cases)

3. **matrix-test** - Multi-platform/version tests
   - OS: Ubuntu, Windows, macOS
   - Rust: stable, beta

4. **extended** - Extended tests (main branch only)
   - Full property tests (1000 cases)
   - Coverage report generation
   - Codecov upload

5. **benchmarks** - Performance tracking (main branch only)
   - Runs all benchmarks
   - Stores results for comparison

### `.github/workflows/nightly.yml` - Nightly Tests

Runs comprehensive testing every night at 2:00 AM UTC.

#### Jobs:

1. **fuzz-tests** - Fuzz testing with cargo-fuzz
   - Targets: parser, value_source, mllp_codec, escape
   - Duration: 5 minutes per target (configurable)

2. **mutation-tests** - Mutation testing with cargo-mutants
   - Tests code coverage quality
   - Identifies untested code paths

3. **extended-property-tests** - Thorough property testing
   - 10,000 test cases per property

4. **security-audit** - Security scanning
   - cargo-audit for vulnerability scanning
   - cargo-deny for license and source checks

5. **docs** - Documentation generation
   - Builds API documentation
   - Uploads as artifact

### `.github/workflows/coverage.yml` - Coverage Reports

Dedicated workflow for code coverage analysis.

#### Jobs:

1. **tarpaulin** - Coverage with cargo-tarpaulin
   - Generates XML and HTML reports
   - Uploads to Codecov

2. **llvm-cov** - Coverage with cargo-llvm-cov
   - Faster alternative to tarpaulin
   - Generates LCOV and HTML reports

3. **coverage-diff** - PR coverage comparison
   - Compares PR coverage against base branch

## Viewing CI Results

### GitHub Actions

1. Navigate to the **Actions** tab in GitHub
2. Select the workflow run you want to view
3. Each job's logs are available by clicking on the job name

### Codecov

Coverage reports are uploaded to Codecov:
- [Codecov Dashboard](https://codecov.io/gh/your-org/hl7v2-rs) (configure with your organization)

### Artifacts

The following artifacts are generated and available for download:

| Workflow | Artifact | Contents |
|----------|----------|----------|
| ci.yml | coverage-report | HTML coverage reports |
| ci.yml | benchmark-results | Benchmark output |
| nightly.yml | fuzz-crash-* | Fuzz test crash inputs |
| nightly.yml | mutation-report | Mutation testing results |
| nightly.yml | api-docs | Generated API documentation |
| coverage.yml | tarpaulin-coverage | Tarpaulin coverage reports |
| coverage.yml | llvm-coverage | LLVM coverage reports |

## Manual Triggers

### Nightly Tests

The nightly workflow can be manually triggered with custom parameters:

```yaml
inputs:
  fuzz_duration: '600'  # Fuzz duration in seconds
  run_mutation: true    # Enable/disable mutation tests
```

To trigger manually:
1. Go to **Actions** > **Nightly Tests**
2. Click **Run workflow**
3. Configure parameters
4. Click **Run workflow**

### Coverage Reports

The coverage workflow can be manually triggered:
1. Go to **Actions** > **Coverage**
2. Click **Run workflow**
3. Choose whether to upload to Codecov
4. Click **Run workflow**

## Caching Strategy

All workflows use `Swatinem/rust-cache@v2` for caching cargo dependencies:

- **Shared keys** are used for similar jobs to maximize cache hits
- Cache keys include OS and Rust version for proper isolation
- Caches are automatically invalidated after7 days

## Concurrency Control

All workflows use concurrency groups to:
- Cancel in-progress runs when a new commit is pushed
- Prevent duplicate runs on the same branch

## Failure Handling

### Fast Fail

- The `fast` job uses `fail-fast: true` to stop immediately on errors
- Matrix tests use `fail-fast: false` to collect all failures

### Continue on Error

The following jobs use `continue-on-error: true`:
- Benchmarks (performance tracking shouldn't block CI)
- Fuzz tests (exploratory testing)
- Mutation tests (informational)

## Required Secrets

| Secret | Purpose | Required For |
|--------|---------|--------------|
| `CODECOV_TOKEN` | Upload coverage to Codecov | Coverage uploads |
| `GITHUB_TOKEN` | GitHub API access | Built-in, automatic |

## Best Practices

### For Contributors

1. **Run fast checks locally before pushing:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --lib --workspace
   cargo test --doc --workspace
   ```

2. **Run integration tests before creating PR:**
   ```bash
   cargo test --test '*' --workspace
   ```

3. **Check property tests locally (limited):**
   ```bash
   PROPTEST_CASES=100 cargo test --workspace --features proptest property
   ```

### For Maintainers

1. **Review nightly test results** regularly for:
   - Fuzz test crashes
   - Mutation test coverage gaps
   - Security advisories

2. **Monitor benchmark results** for performance regressions

3. **Update workflow versions** quarterly:
   - Actions (e.g., `actions/checkout@v4`)
   - Rust toolchain
   - Cargo tools (tarpaulin, llvm-cov, etc.)

## Troubleshooting

### Common Issues

1. **Cache miss:** First run on a new branch will be slower
2. **Timeout:** Large PRs may need increased timeout
3. **Flaky tests:** Check for race conditions, especially in async code

### Debug Mode

Enable debug logging by setting repository variable:
- `ACTIONS_RUNNER_DEBUG` = `true`
- `ACTIONS_STEP_DEBUG` = `true`

## Related Documentation

- [Testing Architecture](./TESTING_ARCHITECTURE.md)
- [Development Guide](../DEVELOPMENT.md)
- [Contributing Guide](../CONTRIBUTING.md)
