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

# --- Scaffolding ---

# Create a new microcrate: just scaffold my-new-feature "Description of feature"
scaffold NAME DESC="":
    cargo run -p xtask -- scaffold {{NAME}} --description "{{DESC}}"

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
