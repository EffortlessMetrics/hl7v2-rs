# Nix Flake Usage Guide

This project uses [Nix flakes](https://nixos.wiki/wiki/Flakes) for reproducible builds and development environments.

## Prerequisites

Install Nix with flakes enabled:

```bash
# Install Nix (if not already installed)
sh <(curl -L https://nixos.org/nix/install) --daemon

# Enable flakes (add to ~/.config/nix/nix.conf or /etc/nix/nix.conf)
experimental-features = nix-command flakes
```

## Quick Start

### Development Shell

Enter the development environment with all tools:

```bash
nix develop
```

This provides:
- **Rust toolchain**: Latest stable with rust-analyzer, clippy, rustfmt
- **Cargo extensions**: watch, edit, audit, outdated, llvm-cov, nextest
- **Schema validation**: Node.js, ajv-cli for YAML schema validation
- **BDD testing**: Cucumber for behavior-driven development
- **Infrastructure tools**: Docker Compose, kubectl, k9s
- **Policy as code**: Open Policy Agent (OPA)
- **Observability**: Prometheus, Grafana
- **Utilities**: jq, yq, just, watchexec

The shell also:
- Sets up git pre-commit hooks automatically
- Displays helpful command reference
- Configures Rust environment variables

### CI Shell (Minimal)

For CI environments, use the minimal shell:

```bash
nix develop .#ci
```

Includes only:
- Rust toolchain
- Build dependencies
- Schema validation (ajv-cli)

### Building

Build all packages:

```bash
nix build
```

Build specific package:

```bash
nix build .#default      # Main package (all binaries)
nix build .#docker       # Docker image
```

### Running

Run the server directly:

```bash
nix run . -- server --host 0.0.0.0 --port 8080
```

### Docker Image

Build Docker image with Nix:

```bash
nix build .#docker
docker load < result
docker run -p 8080:8080 hl7v2-rs:latest
```

### Checks

Run all checks (format, clippy, tests):

```bash
nix flake check
```

Individual checks:

```bash
nix build .#checks.x86_64-linux.format  # Format check
nix build .#checks.x86_64-linux.clippy  # Linting
nix build .#checks.x86_64-linux.test    # Tests
```

## Flake Outputs

### Packages

- **`packages.default`**: Main Rust package with all binaries
  - `hl7v2-server` - HTTP/REST API server
  - `hl7v2-cli` - Command-line interface (future)
- **`packages.docker`**: Layered Docker image optimized for caching

### Development Shells

- **`devShells.default`**: Full development environment with all tools
- **`devShells.ci`**: Minimal environment for CI/CD pipelines

### Checks

- **`checks.build`**: Build succeeds
- **`checks.format`**: Code is properly formatted
- **`checks.clippy`**: No clippy warnings
- **`checks.test`**: All tests pass

## Reproducibility

### Pinned Dependencies

The flake pins all dependencies via `flake.lock`:

```bash
# Update all inputs
nix flake update

# Update specific input
nix flake update nixpkgs
nix flake update rust-overlay
```

### Rust Toolchain

Uses [rust-overlay](https://github.com/oxalica/rust-overlay) for precise Rust version control:

```nix
rustToolchain = pkgs.rust-bin.stable.latest.default.override {
  extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
};
```

### Build Inputs

Native dependencies are explicitly listed:

- **Native build inputs**: `rustToolchain`, `pkg-config`, `openssl`
- **Build inputs**: `openssl`
- **Darwin-specific**: `Security`, `CoreFoundation` frameworks

## Cross-Platform Support

The flake supports:
- **Linux**: x86_64-linux, aarch64-linux
- **macOS**: x86_64-darwin, aarch64-darwin (Apple Silicon)

Darwin builds include necessary Apple SDK frameworks.

## Pre-Commit Hooks

The development shell automatically installs git pre-commit hooks that run:

1. **Format check**: `cargo fmt --all -- --check`
2. **Linting**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests**: `cargo test --all`
4. **Schema validation**: Validates YAML profiles against JSON schema (if available)

Hooks are installed on first `nix develop` entry.

## CI/CD Integration

### GitHub Actions

```yaml
- name: Setup Nix
  uses: cachix/install-nix-action@v22
  with:
    extra_nix_config: |
      experimental-features = nix-command flakes

- name: Build
  run: nix build

- name: Run checks
  run: nix flake check
```

### GitLab CI

```yaml
image: nixos/nix:latest

before_script:
  - nix --version

build:
  script:
    - nix build
    - nix flake check
```

## Advanced Usage

### Direnv Integration

For automatic environment loading with [direnv](https://direnv.net/):

```bash
# .envrc
use flake
```

Then run:

```bash
direnv allow
```

Now `cd`-ing into the directory automatically loads the Nix environment.

### Custom Rust Version

To use a specific Rust version, edit `flake.nix`:

```nix
rustToolchain = pkgs.rust-bin.stable."1.91.0".default.override {
  extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
};
```

Or use nightly:

```nix
rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
  extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
};
```

### Adding Development Tools

To add tools to the development shell, edit `devTools` in `flake.nix`:

```nix
devTools = with pkgs; [
  # ... existing tools ...

  # Add your tools
  ripgrep
  fd
  bat
];
```

## Troubleshooting

### Flake Evaluation Errors

```bash
# Check flake syntax
nix flake check --show-trace

# Show flake info
nix flake show

# Show flake metadata
nix flake metadata
```

### Build Failures

```bash
# Build with verbose output
nix build --print-build-logs

# Build with trace on error
nix build --show-trace
```

### Garbage Collection

Nix stores all builds in `/nix/store`. Clean up old builds:

```bash
# Remove build artifacts not referenced by any profile
nix-collect-garbage

# Remove all generations older than 7 days
nix-collect-garbage --delete-older-than 7d

# Aggressive cleanup (removes all old generations)
nix-collect-garbage -d
```

## Benefits

### For Developers

✅ **Instant setup**: `nix develop` provides complete environment
✅ **Reproducible**: Same environment on all machines
✅ **Isolated**: No global package pollution
✅ **Versioned**: Environment defined in version control
✅ **Fast iterations**: Cached builds across team

### For CI/CD

✅ **Binary caching**: Shared cache across builds
✅ **Deterministic**: Bit-for-bit reproducible builds
✅ **Cacheable**: Nix cache can be shared via Cachix
✅ **Fast**: Only rebuild what changed
✅ **Auditable**: Cryptographic hashes for all dependencies

### For Production

✅ **Minimal images**: Docker images only contain runtime dependencies
✅ **Layered caching**: Optimal Docker layer structure
✅ **No runtime surprises**: Same dependencies as dev/CI
✅ **Security**: Clear dependency provenance

## References

- [Nix Flakes Guide](https://nixos.wiki/wiki/Flakes)
- [Rust Overlay Documentation](https://github.com/oxalica/rust-overlay)
- [Nix Pills](https://nixos.org/guides/nix-pills/) - In-depth Nix tutorial
- [Zero to Nix](https://zero-to-nix.com/) - Gentle introduction to Nix
- [Nix Package Search](https://search.nixos.org/packages) - Find packages

## License

This flake configuration is part of the hl7v2-rs project and is licensed under AGPL-3.0-or-later.
