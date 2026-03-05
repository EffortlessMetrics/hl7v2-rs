# Justfile for common development tasks
# Install just: https://github.com/casey/just

# List all available commands
default:
    @just --list

# Build all crates
build:
    cargo build --all

# Build in release mode
build-release:
    cargo build --release --all

# Run all tests
test:
    cargo test --all

# Run tests with output
test-verbose:
    cargo test --all -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --exact --nocapture

# Run benchmarks
bench:
    cargo bench --all

# Run specific benchmark
bench-one BENCH:
    cargo bench -- {{BENCH}}

# Fix code formatting and lints
lint-fix:
    cargo run -p xtask -- lint-fix

# Run gate checks (format, clippy, test)
gate:
    cargo run -p xtask -- gate

# Setup development environment
setup:
    cargo run -p xtask -- setup

# Check code with clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run all checks (format, lint, test)
check: gate
    @echo "✅ All checks passed!"

# Generate code coverage
coverage:
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
    cargo llvm-cov --all-features --workspace --html
    @echo "Coverage report generated in target/llvm-cov/html/index.html"

# Watch for changes and rebuild
watch:
    cargo watch -x build

# Watch and run tests
watch-test:
    cargo watch -x test

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/
    rm -f lcov.info

# Validate YAML against schemas
validate-schemas:
    ajv validate -s schemas/profile/profile-v1.schema.json -d 'profiles/*.yaml'
    ajv validate -s schemas/config/hl7v2-config-v1.schema.json -d 'config/*.toml' || true
    @echo "✅ Schema validation complete"

# Run BDD tests
bdd:
    cargo test --features cucumber --test bdd_tests

# Generate test corpus
gen-corpus SEED="42" COUNT="100":
    cargo run -p hl7v2-cli -- gen \
        --template templates/adt_a01.yaml \
        --seed {{SEED}} \
        --count {{COUNT}} \
        --out test_data/corpus/

# Verify corpus manifest
verify-corpus:
    cargo run -p hl7v2-cli -- gen --verify-manifest test_data/corpus/manifest.json

# Run local development stack
dev-up:
    docker-compose -f infrastructure/docker-compose.yml up -d

# Stop local development stack
dev-down:
    docker-compose -f infrastructure/docker-compose.yml down

# View logs from development stack
dev-logs:
    docker-compose -f infrastructure/docker-compose.yml logs -f

# Run server locally
run-server PORT="8080":
    cargo run -p hl7v2-cli -- server --port {{PORT}}

# Run MLLP server
run-mllp PORT="2575":
    cargo run -p hl7v2-cli -- server --mllp --port {{PORT}}

# Parse a message
parse FILE:
    cargo run -p hl7v2-cli -- parse {{FILE}} --json

# Validate a message
validate FILE PROFILE:
    cargo run -p hl7v2-cli -- val {{FILE}} --profile {{PROFILE}}

# Build Docker image
docker-build:
    docker build -t hl7v2-rs:latest -f infrastructure/Dockerfile .

# Run Docker image
docker-run:
    docker run -p 8080:8080 -p 2575:2575 hl7v2-rs:latest

# Build with Nix
nix-build:
    nix build

# Check Nix flake
nix-check:
    nix flake check

# Update Nix dependencies
nix-update:
    nix flake update

# Deploy to Kubernetes
k8s-deploy:
    kubectl apply -f infrastructure/k8s/

# Check Kubernetes deployment
k8s-status:
    kubectl get pods,svc,deploy -l app=hl7v2-server

# Tail Kubernetes logs
k8s-logs:
    kubectl logs -f -l app=hl7v2-server

# Run security audit
audit:
    cargo audit

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# Install git hooks
install-hooks: setup

# CI: Run all CI checks
ci: gate validate-schemas
    @echo "✅ CI checks complete"

# Release: Prepare a new release
release VERSION:
    #!/usr/bin/env bash
    set -e
    echo "Preparing release {{VERSION}}"

    # Update version in Cargo.toml files
    sed -i 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    sed -i 's/^version = ".*"/version = "{{VERSION}}"/' crates/*/Cargo.toml

    # Update Cargo.lock
    cargo update -p hl7v2-core -p hl7v2-prof -p hl7v2-gen -p hl7v2-cli

    # Run checks
    just ci

    # Create git tag
    git add Cargo.toml crates/*/Cargo.toml Cargo.lock
    git commit -m "chore: bump version to {{VERSION}}"
    git tag -a "v{{VERSION}}" -m "Release {{VERSION}}"

    echo "✅ Release {{VERSION}} prepared"
    echo "Next steps:"
    echo "  git push origin main"
    echo "  git push origin v{{VERSION}}"
