# ADR-0005: Nix for Reproducible Builds

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Development and CI environments exhibited inconsistencies across machines due to differing Rust toolchain versions, system library versions, and tool availability. A healthcare integration system demands reproducible builds to satisfy regulatory and operational requirements.

## Context

Building a healthcare integration system introduces several environmental challenges:

1. **Toolchain drift**: Developers on different machines may have different Rust versions. The project targets Rust edition 2024 with MSRV 1.92, and even minor version differences can cause compilation failures or behavioral changes.

2. **System library dependencies**: The project depends on OpenSSL for TLS support in MLLP and HTTP servers. OpenSSL versions and linking behavior vary across Linux distributions, macOS (which ships LibreSSL), and Windows.

3. **Tool sprawl**: The development workflow requires 20+ tools beyond the Rust compiler: cargo extensions (watch, edit, audit, outdated, llvm-cov, nextest, expand, deny), schema validation (nodejs, ajv-cli), policy enforcement (open-policy-agent), BDD testing (cucumber), infrastructure management (docker-compose, kubectl, k9s), observability (prometheus, grafana), and utilities (jq, yq-go, just, watchexec, gh).

4. **CI/local parity**: Differences between local development environments and CI cause "works on my machine" failures. The CI pipeline runs formatting, clippy, tests, and schema validation, and all of these must pass locally before pushing.

5. **macOS portability**: macOS requires additional frameworks (Security, CoreFoundation) for TLS, and these must be conditionally included without breaking Linux builds.

6. **Container builds**: The project produces Docker images exposing HTTP (port 8080) and MLLP (port 2575) endpoints. Container image construction must be reproducible and minimal.

## Decision

We will use **Nix flakes** (`flake.nix`) as the single source of truth for development environments, CI dependencies, and container image builds.

The flake provides:

1. **Pinned Rust toolchain** via `oxalica/rust-overlay` with extensions: `rust-src`, `rust-analyzer`, `clippy`, `rustfmt`.
2. **Two development shells**: `devShells.default` (full development with 20+ tools) and `devShells.ci` (minimal CI with only build dependencies and ajv-cli).
3. **Reproducible Docker images** via `pkgs.dockerTools.buildLayeredImage`.
4. **Flake checks** (`nix flake check`) that mirror CI: build, format, clippy, and test.

**Rationale:**

1. **Single-command setup**: `nix develop` provides a complete development environment with no manual installation steps.
2. **Exact reproducibility**: Nix's content-addressed store and lock file (`flake.lock`) ensure every developer and CI runner uses identical tool versions.
3. **Declarative specification**: All dependencies are declared in one file, making the environment auditable and reviewable.
4. **Cross-platform**: Nix handles platform-specific dependencies (macOS frameworks, Linux system libraries) through conditional expressions.
5. **CI parity**: The `devShells.ci` shell provides exactly what CI needs, and `nix flake check` runs the same gates locally.

## Consequences

### Positive

- **Eliminated environment drift**: Every developer gets identical Rust toolchain, OpenSSL, and tool versions regardless of their host OS or existing system packages.
- **Fast onboarding**: New contributors run `nix develop` and immediately have a working environment with all 20+ required tools.
- **CI/local parity**: `nix flake check` runs the same build, format, clippy, and test checks that CI runs, catching failures before push.
- **Minimal Docker images**: `dockerTools.buildLayeredImage` produces images containing only the compiled binary and its runtime dependencies, without a base OS layer or build tools.
- **Auditable supply chain**: `flake.lock` pins every transitive dependency to exact content hashes, providing a complete software bill of materials.
- **Platform abstraction**: The `eachDefaultSystem` helper generates outputs for all supported platforms (x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin) from a single definition.

### Negative

- **Nix learning curve**: Nix's functional language and concepts (derivations, overlays, flakes) have a steep learning curve for developers unfamiliar with the ecosystem.
- **Disk usage**: Nix stores every dependency version independently in `/nix/store`, consuming more disk space than shared system libraries.
- **Initial build time**: The first `nix develop` invocation downloads and builds all dependencies, which can take significant time on a fresh machine.
- **Windows limitations**: Nix does not natively support Windows. Windows developers must use WSL2, which adds a layer of indirection.
- **Nix installation required**: Unlike Makefiles or shell scripts, Nix itself must be installed, which requires root access or a user-level install.

### Neutral

- **Parallel to Cargo**: Cargo already manages Rust dependencies. Nix manages everything else (system libraries, non-Rust tools, container images). The two systems coexist without conflict.
- **Pre-commit hooks**: The shell hook installs pre-commit hooks that run `cargo fmt`, `cargo clippy`, `cargo test`, and `ajv validate`. This overlaps with the repository's `.githooks` setup but ensures hooks are present even if `just setup` was not run.

## Alternatives Considered

### Alternative 1: Docker-Only Development

**Pros:**
- Widely adopted and understood
- Works on Windows, macOS, and Linux
- Provides environment isolation
- Simpler mental model than Nix

**Cons:**
- Volume mounts for source code add filesystem overhead, especially on macOS
- IDE integration (rust-analyzer, debugger attachment) is more difficult inside containers
- Docker-in-Docker is needed for container image builds during development
- Rebuilding the development image after dependency changes is slow
- Docker images are not content-addressed; rebuilding from the same Dockerfile can produce different results

**Why not chosen:**
The developer experience cost of running IDE tooling inside Docker containers is too high. Volume mount performance on macOS degrades compile times significantly. Nix provides the same isolation guarantees while keeping tools native to the host.

### Alternative 2: Makefiles + Manual Tool Installation

**Pros:**
- No additional tooling required beyond `make`
- Familiar to most developers
- Works everywhere
- Zero learning curve

**Cons:**
- Cannot enforce tool versions; relies on each developer installing the right versions
- No mechanism to handle platform-specific dependencies declaratively
- "Works on my machine" is the inevitable outcome
- Maintaining install instructions for 20+ tools across Linux, macOS, and Windows is a documentation burden
- No reproducible container image building

**Why not chosen:**
With 20+ required tools and strict version requirements for a healthcare system, manual installation is error-prone and unenforceable. A single `nix develop` command replaces pages of setup documentation.

### Alternative 3: asdf/rtx (mise) Version Manager

**Pros:**
- Simpler than Nix; `.tool-versions` file specifies versions
- Growing ecosystem of plugins
- Easier learning curve
- Familiar to polyglot developers

**Cons:**
- Cannot manage system libraries (OpenSSL, macOS frameworks)
- No Docker image building capability
- Plugins are community-maintained and may lag behind releases
- Cannot provide CI-specific minimal environments
- No content-addressed caching; downloads are not verified against hashes

**Why not chosen:**
asdf/mise manages language runtimes but not system libraries. Since the project depends on OpenSSL and platform-specific frameworks, asdf alone cannot guarantee reproducibility. We would still need another tool for system dependencies and container builds.

### Alternative 4: GitHub Actions Containers

**Pros:**
- CI environment is defined in workflow YAML
- No local tool installation needed for CI
- GitHub-managed, maintained, and updated
- Good integration with GitHub features

**Cons:**
- Only solves CI reproducibility, not local development
- Developers still need to manually install tools locally
- No mechanism to share environment definitions between CI and local
- Cannot build Docker images reproducibly
- Vendor lock-in to GitHub Actions

**Why not chosen:**
This only addresses half the problem. Local development environments would remain uncontrolled. Nix flakes provide a single definition that serves both local and CI needs.

## Implementation Notes

### Flake Structure

The `flake.nix` is organized into four outputs:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system: {
      packages.default = /* Rust build */;
      packages.docker  = /* Docker image */;
      devShells.default = /* Full dev shell */;
      devShells.ci      = /* Minimal CI shell */;
      checks = /* Format, clippy, test */;
    });
}
```

### Pinned Rust Toolchain

The Rust toolchain is pinned via `rust-overlay` with MSRV 1.92 parity:

```nix
rustToolchain = pkgs.rust-bin.stable.latest.default.override {
  extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
};
```

This ensures all developers and CI use the same compiler version with identical extensions.

### Development Shell Tools

The full development shell includes tools grouped by purpose:

```nix
devTools = with pkgs; [
  # Rust ecosystem
  cargo-watch cargo-edit cargo-audit cargo-outdated
  cargo-llvm-cov cargo-nextest cargo-expand cargo-deny

  # Schema validation
  nodejs nodePackages.ajv-cli

  # Policy enforcement
  open-policy-agent

  # BDD testing
  cucumber

  # Infrastructure
  docker-compose kubectl k9s

  # Observability
  prometheus grafana

  # Utilities
  jq yq-go just watchexec gh
];
```

### CI Shell

The CI shell is deliberately minimal to reduce Nix evaluation and download time in pipelines:

```nix
devShells.ci = pkgs.mkShell {
  buildInputs = nativeBuildInputs ++ buildInputs ++ [
    pkgs.nodePackages.ajv-cli
  ];
};
```

It includes only the Rust toolchain, OpenSSL, pkg-config, and ajv-cli for schema validation.

### Docker Image Building

Container images are built using Nix's `dockerTools.buildLayeredImage`, which produces layered images without a base OS:

```nix
packages.docker = pkgs.dockerTools.buildLayeredImage {
  name = "hl7v2-rs";
  tag = "latest";
  contents = [ self.packages.${system}.default ];
  config = {
    Cmd = [ "${self.packages.${system}.default}/bin/hl7v2-server" ];
    ExposedPorts = {
      "8080/tcp" = {};  # HTTP API
      "2575/tcp" = {};  # MLLP
    };
    Env = [
      "RUST_LOG=info"
      "HL7V2_HOST=0.0.0.0"
      "HL7V2_PORT=8080"
    ];
  };
};
```

This approach produces smaller, more secure images than Dockerfile-based builds because the image contains only the compiled binary and its runtime closure.

### Flake Checks

`nix flake check` runs four checks that mirror CI:

```nix
checks = {
  build  = self.packages.${system}.default;
  format = /* cargo fmt --all -- --check */;
  clippy = /* cargo clippy --all-targets --all-features -- -D warnings */;
  test   = /* cargo test --all */;
};
```

### Platform-Specific Dependencies

macOS frameworks are conditionally included:

```nix
buildInputs = with pkgs; [
  openssl
] ++ lib.optionals stdenv.isDarwin [
  darwin.apple_sdk.frameworks.Security
  darwin.apple_sdk.frameworks.CoreFoundation
];
```

## References

- [Nix Flakes](https://nixos.wiki/wiki/Flakes) -- Flake specification and usage
- [rust-overlay](https://github.com/oxalica/rust-overlay) -- Nix overlay for pinned Rust toolchains
- [flake-utils](https://github.com/numtide/flake-utils) -- Helper for multi-platform flake outputs
- [dockerTools](https://nixos.org/manual/nixpkgs/stable/#sec-pkgs-dockerTools) -- Nix-native Docker image building
- [Nix Pills](https://nixos.org/guides/nix-pills/) -- Introduction to Nix concepts
- [Reproducible Builds](https://reproducible-builds.org/) -- Industry initiative for build reproducibility
- Internal: `flake.nix` -- Flake definition (227 lines)
- Internal: `flake.lock` -- Pinned dependency hashes
- Internal: `.githooks/` -- Repository hook scripts complementing Nix shell hooks
