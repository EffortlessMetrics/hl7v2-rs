{
  description = "HL7v2-rs - Modern Rust HL7v2 Processor with reproducible builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        workspaceVersion = cargoToml.workspace.package.version;
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain for Nix builds.
        # Note: Using stable Rust toolchain from rust-overlay
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # Native build inputs
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          openssl
        ];

        # Build inputs
        buildInputs = with pkgs; [
          openssl
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.CoreFoundation
        ];

        # Development tools
        devTools = with pkgs; [
          # Rust tools
          cargo-watch
          cargo-edit
          cargo-audit
          cargo-outdated
          cargo-llvm-cov
          cargo-nextest
          cargo-expand
          cargo-deny

          # Schema validation
          nodejs
          nodePackages.ajv-cli

          # BDD testing
          cucumber

          # Infrastructure tools
          docker-compose
          kubectl
          k9s

          # Policy as code
          open-policy-agent

          # Observability
          prometheus
          grafana

          # Development utilities
          jq
          yq-go
          just
          watchexec
          gh
        ];

      in
      {
        # Default package
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hl7v2-rs";
          version = workspaceVersion;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Run tests during build
          checkPhase = ''
            cargo test --release --all
          '';

          meta = with pkgs.lib; {
            description = "Modern Rust HL7v2 Processor";
            homepage = "https://github.com/EffortlessMetrics/hl7v2-rs";
            license = licenses.agpl3Plus;
            maintainers = [ ];
          };
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = nativeBuildInputs ++ buildInputs ++ devTools;

          shellHook = ''
            echo "🏥 HL7v2-rs Development Environment"
            echo ""
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build       - Build the project"
            echo "  cargo test        - Run tests"
            echo "  cargo bench       - Run benchmarks"
            echo "  cargo clippy      - Lint code"
            echo "  cargo fmt         - Format code"
            echo "  cargo watch       - Watch for changes and rebuild"
            echo "  just <task>       - Run justfile tasks"
            echo ""
            echo "Schema validation:"
            echo "  ajv validate -s schemas/profile/profile-v1.schema.json -d 'examples/profiles/*.yaml'"
            echo ""
            echo "Infrastructure:"
            echo "  docker-compose up - Start local development stack"
            echo "  kubectl apply -f infrastructure/k8s/ - Deploy to Kubernetes"
            echo ""

            # Use the tracked repository hooks, which delegate to xtask.
            # This keeps Nix and non-Nix development on one hook workflow and
            # also works in linked worktrees where .git is a file.
            if REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" &&
              [ -f "$REPO_ROOT/.githooks/pre-commit" ] &&
              [ -f "$REPO_ROOT/.githooks/pre-push" ]; then
              if chmod +x "$REPO_ROOT"/.githooks/pre-commit "$REPO_ROOT"/.githooks/pre-push &&
                git -C "$REPO_ROOT" config extensions.worktreeConfig true &&
                git -C "$REPO_ROOT" config --worktree core.hooksPath .githooks; then
                echo "✅ Using repository hooks from .githooks"
              else
                echo "⚠️ Could not configure repository hooks from .githooks" >&2
              fi
            fi
          '';

          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          RUST_BACKTRACE = "1";
        };

        # CI shell (minimal, fast)
        devShells.ci = pkgs.mkShell {
          buildInputs = nativeBuildInputs ++ buildInputs ++ [
            pkgs.nodePackages.ajv-cli
          ];
        };

        # Docker image
        packages.docker = pkgs.dockerTools.buildLayeredImage {
          name = "hl7v2-rs";
          tag = workspaceVersion;

          contents = [ self.packages.${system}.default ];

          config = {
            Cmd = [ "${self.packages.${system}.default}/bin/hl7v2-server" ];
            ExposedPorts = {
              "8080/tcp" = {};  # HTTP API
              "2575/tcp" = {};  # MLLP (if implemented)
            };
            Env = [
              "RUST_LOG=info"
              "HL7V2_HOST=0.0.0.0"
              "HL7V2_PORT=8080"
            ];
          };
        };

        # Checks (run with `nix flake check`)
        checks = {
          build = self.packages.${system}.default;

          format = pkgs.runCommand "check-format" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${./.}
            cargo fmt --all -- --check
            touch $out
          '';

          clippy = pkgs.runCommand "check-clippy" {
            buildInputs = [ rustToolchain ] ++ buildInputs;
          } ''
            cd ${./.}
            cargo clippy --all-targets --all-features -- -D warnings
            touch $out
          '';

          test = pkgs.runCommand "check-tests" {
            buildInputs = [ rustToolchain ] ++ buildInputs;
          } ''
            cd ${./.}
            cargo test --all
            touch $out
          '';
        };
      }
    );
}
