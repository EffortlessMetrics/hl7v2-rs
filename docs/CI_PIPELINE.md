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
   - Format check (`cargo run -p xtask -- gate --check --only fmt`)
   - Clippy lints (`cargo run -p xtask -- gate --check --only clippy`)
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

5. **benchmarks** - Routed performance tracking (main branch or label)
   - Runs the `hl7v2-bench` harness with a bounded Criterion sample
   - Stores results for comparison
   - Fails when the routed benchmark command fails

6. **ci-success** - Required CI summary
   - Aggregates required fast, standard, MSRV, and matrix lane results
   - Allows the routed matrix lane to skip cleanly when not selected
   - Leaves optional deep lanes such as extended tests and benchmarks to report
     through their own jobs

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

The `nightly-summary` job runs after the main nightly proof jobs and fails the
workflow when any required nightly job reports `failure`. This keeps scheduled
deep-verification failures visible while still allowing the summary step to run
with `if: always()`.

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

### `.github/workflows/droid.yml` - Factory Droid Manual Review

Optional Factory Droid review automation. It responds only to explicit `@droid`
commands in pull request comments or review text. Automatic PR review is not
enabled, so this workflow does not add a status check to every PR.

#### Setup:

1. Install the Factory Droid GitHub App for this repository.
2. Add `FACTORY_API_KEY` under repository or organization Actions secrets.
3. Ask a repository writer to comment `@droid review`, `@droid fill`, or
   `@droid security` on a pull request.

Full-repository `@droid security --full` scans are not enabled in this first
pass because they can create branches and need broader write permissions.

### `.github/workflows/python-wheels.yml` - Python Wheel Smoke

Builds the public `hl7v2` Python distribution from the `hl7v2-python`
binding backend crate and proves the wheel can be installed and
imported. This workflow does not publish to PyPI.

#### Jobs:

1. **wheel-smoke** - Python package proof
   - Builds a wheel with maturin
   - Installs the built wheel
   - Runs `tests/python_smoke/smoke.py`
   - Runs `tests/python_smoke/evidence_workflow_guide.py`
   - Runs `tests/python_smoke/dirty_evidence_workflow.py`
   - Uploads the wheel as a short-retention smoke artifact

### `.github/workflows/python-testpypi.yml` - Python TestPyPI Proof

Manual-only workflow for the separate `hl7v2` Python distribution lane. The
default dispatch builds the wheel, installs it into a fresh virtual
environment, runs `tests/python_smoke/smoke.py` plus the Python evidence guide
and dirty evidence workflow smokes, and uploads the wheel as a short-retention
artifact without publishing.

When `publish_to_testpypi=true` is selected, the workflow publishes to TestPyPI
using Trusted Publishing from the `testpypi` GitHub environment, then installs
`hl7v2==<workspace version>` back from TestPyPI and reruns the smoke and
evidence workflows. The workflow uses `id-token: write` only for the publish
job and does not use repository PyPI tokens. Publishing mode fails early unless
the workflow is running from `refs/heads/main`.

### `.github/workflows/python-pypi.yml` - Python PyPI Release Proof

Manual-only workflow for the production `hl7v2` Python distribution lane. The
default dispatch builds the wheel, installs it into a fresh virtual
environment, runs `tests/python_smoke/smoke.py` plus the Python evidence guide
and dirty evidence workflow smokes, and uploads the wheel as a short-retention
artifact without publishing.

When `publish_to_pypi=true` is selected, the workflow publishes to production
PyPI using Trusted Publishing from the `pypi` GitHub environment, then installs
`hl7v2==<workspace version>` back from PyPI and reruns all three smoke
workflows.
The workflow uses `id-token: write` only for the publish job and does not use
repository PyPI tokens. Run the TestPyPI publishing proof first; production PyPI
is not a substitute for TestPyPI proof. Publishing mode fails early unless the
workflow is running from `refs/heads/main`, the supplied TestPyPI proof URL
points to the same source commit, and that proof's publish plus install-back
jobs succeeded.

The local policy rail is:

```powershell
cargo run -p xtask -- check-python-publish-policy
```

It verifies that `pyproject.toml` names the public Python distribution `hl7v2`,
points maturin at `crates/hl7v2-python/Cargo.toml` with the public Python module
name `hl7v2`, the
TestPyPI/PyPI workflows remain manual, default to non-publishing mode, publish
only from `main`, run all three Python smoke scripts before upload and during
install-back, reject `skip-existing`, avoid secret-backed upload credentials,
grant `id-token: write` only on publish jobs, and keep the `hl7v2-python`
binding backend crate outside the primary Rust product graph until a separate
binding-backend release PR changes that policy.

### Markdown Local-Link Policy

The local documentation-link rail is:

```powershell
cargo run -p xtask -- check-doc-links
```

It scans tracked and untracked non-ignored Markdown files outside
generated/vendor directories and fails when explicit relative local links point
at missing repository targets, escape the workspace, or rely on
case-insensitive path matching. The full `xtask gate --check` path runs this
after the non-Rust file policy check, which also scans tracked and untracked
non-ignored files, so release and audit receipts do not depend on an ad hoc
local script. `xtask gate --check --changed` also runs this rail for
crate-scoped Markdown changes so a broken link in a crate README cannot bypass
the changed-crate shortcut.

### `.github/workflows/server-smoke.yml` - Server Docker Smoke

Path-scoped and manual workflow for the deployable validation sidecar. It
validates `infrastructure/docker/docker-compose.yml`, starts the checked-in
Compose stack, runs `tests/server_smoke/smoke.py`, prints container logs on
failure, and tears the stack down with volumes removed.

The smoke script exercises `/health`, `/ready`, `/hl7/validate-redacted`,
`/hl7/bundle`, `/hl7/replay`, and `/hl7/corpus/diff` against the running
container. It uses the local `dev-secret` API key from
`infrastructure/docker/sidecar.env.example`; do not replace that with a real
deployment secret. The Compose service is also CPU/memory bounded so local
sidecar proof cannot consume unlimited host resources.

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
| python-wheels.yml | python-wheel-* | Smoke-test wheels for the Python binding lane |
| python-testpypi.yml | python-testpypi-wheel | Manual TestPyPI proof wheel artifact |
| python-pypi.yml | python-pypi-wheel | Manual production PyPI release proof wheel artifact |

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

### Server Docker Smoke

The server smoke workflow can be manually triggered when validating sidecar
deployment behavior:

1. Go to **Actions** > **Server Docker Smoke**
2. Click **Run workflow**
3. Select the branch to test
4. Click **Run workflow**

The same workflow also runs on PRs and `main` pushes that change the server,
canonical library, Docker sidecar files, profiles, smoke script, or sidecar
deployment guide.

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

Some exploratory or advisory steps use `continue-on-error: true` inside their
jobs:
- Fuzz tests (exploratory testing)
- Mutation tests (informational)

Nightly job failures that reach the workflow dependency graph are aggregated by
the `nightly-summary` job and fail the workflow instead of being downgraded to
warnings.

Benchmarks are not a default ordinary-PR lane, but once the benchmark lane is
routed on `main`, manual dispatch, or a labeled PR, benchmark command failures
fail the `Benchmarks` job. They are not aggregated into `CI Success`, so
routing benchmarks does not change the required branch-protection summary. CI
uses a bounded Criterion sample to prove the benchmark harness still compiles
and runs; maintainers can run longer local benchmark sessions when comparing
performance in detail.

## Required Secrets

| Secret | Purpose | Required For |
|--------|---------|--------------|
| `CODECOV_TOKEN` | Upload coverage to Codecov | Coverage uploads |
| `FACTORY_API_KEY` | Authenticate Factory Droid sessions | Optional Droid manual review workflow |
| `GITHUB_TOKEN` | GitHub API access | Built-in, automatic |

## Best Practices

### For Contributors

1. **Run fast checks locally before pushing:**
   ```bash
   cargo run -p xtask -- gate --check
   # or equivalently:
   just gate-check
   ```

2. **Run integration tests before creating PR:**
   ```bash
   cargo test --test '*' --workspace
   ```

3. **Check property tests locally (limited):**
   ```bash
   PROPTEST_CASES=100 cargo test --workspace --test property_tests -- --nocapture
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
