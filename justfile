# Justfile for common development tasks
# Install just: https://github.com/casey/just

# List all available commands
default:
    @just --list

# --- Basic Workflow ---

# One-time repository setup (hooks, etc.)
setup:
    cargo run -p xtask -- setup

# Run all formatting and clippy fixes (mutating)
lint-fix:
    cargo run -p xtask -- lint-fix

# Check formatting and lints (non-mutating)
lint-check:
    cargo run -p xtask -- gate --only clippy

# Run the local "CI preview" gate (fast)
gate:
    cargo run -p xtask -- gate

# Run the strict gate (CI parity)
gate-check:
    cargo run -p xtask -- gate --check

# Only check changed crates
gate-changed:
    cargo run -p xtask -- gate --changed

# --- Documentation ---

# Generate and open documentation
docs:
    cargo run -p xtask -- docs

# Generate documentation without opening
docs-build:
    cargo run -p xtask -- docs --no-open

# --- Quality & Security ---

# Run security audit and license check
audit:
    cargo run -p xtask -- audit

# Check for outdated dependencies
outdated:
    cargo run -p xtask -- outdated

# Print crates.io publish order derived from workspace metadata
publish-plan:
    cargo run -p xtask -- publish-plan

# --- Policy stack ---

# Verify lint, no-panic-family, non-Rust file, doc-link, and Python publish policies (CI parity)
policy-check:
    cargo run -p xtask -- check-lint-policy
    cargo run -p xtask -- check-no-panic-family
    cargo run -p xtask -- check-file-policy
    cargo run -p xtask -- check-doc-links
    cargo run -p xtask -- check-python-publish-policy

# Print policy rollout, debt, no-panic, and file-policy summary
policy-report:
    cargo run -p xtask -- policy-report

# Generate proposed no-panic allowlist entries from current findings
no-panic-propose:
    cargo run -p xtask -- no-panic propose

# Run tests with nextest (faster)
test:
    @if command -v cargo-nextest > /dev/null; then \
        cargo nextest run --workspace --all-features; \
    else \
        cargo test --workspace --all-features; \
    fi

# Run benchmarks
bench:
    cargo bench --workspace

# --- Utilities ---

# Clean build artifacts
clean:
    cargo clean

# Run local development stack
dev-up:
    docker-compose -f infrastructure/docker/docker-compose.yml up -d

# Stop local development stack
dev-down:
    docker-compose -f infrastructure/docker/docker-compose.yml down

# CI: Run all CI checks
ci: gate-check audit docs-build
    @echo "✅ CI checks complete"
