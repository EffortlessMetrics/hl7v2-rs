# Non-Rust File Policy

`hl7v2-rs` is a Rust-first repository. Non-Rust programming and configuration
surfaces are governed: every such file must have an entry in
`policy/non-rust-allowlist.toml` describing **why** the surface exists, who
owns it, and how it is verified.

The policy is enforced by `cargo run -p xtask -- check-file-policy`.

## Auto-allowed surfaces

These surfaces do not require allowlist entries because they are intrinsic to
a Rust workspace:

```text
*.rs                    Rust source
Cargo.toml, Cargo.lock  Cargo manifests and lockfile
*.md                    Markdown documentation
LICENSE, NOTICE         license files
.gitignore              git metadata
.gitattributes          git metadata
```

Everything else in the tracked or untracked non-ignored source tree that is a
programming or configuration surface needs an explicit entry.

## Allowlist schema

`policy/non-rust-allowlist.toml` uses schema `1.0`:

```toml
schema_version = "1.0"

[[allow]]
glob = ".github/workflows/*.yml"
kind = "ci_declarative"
owner = "release/ci"
surface = "ci"
classification = "config"
reason = "GitHub Actions workflow definitions are platform-required YAML."
covered_by = ["cargo run -p xtask -- gate --check"]
```

Required fields: `glob` **or** `path`, `kind`, `owner`, `surface`,
`classification`, `reason`. `covered_by` is required for `production`,
`test`, and `tooling` classifications. `expires` is optional; if set, the
checker fails when the date is past.

`classification` is one of:

```text
production    surface that ships or runs in production / users see it
test          surface that drives tests, fixtures, or test scaffolding
tooling       surface that supports build/release/CI but does not ship
config        repo-level configuration (clippy, deny, codecov, etc.)
generated     checked-in artifacts produced by a generator
docs          docs-only surface beyond standard markdown
```

Common `kind` values used in this repo:

```text
ci_declarative           GitHub Actions, codecov, etc.
git_hook                 .githooks/*
fixture_input            test corpora, snapshots, proptest regressions
bdd_feature              .feature files for cucumber/BDD
proto_idl                .proto files
openapi_spec             OpenAPI definitions
profile_data             HL7 profile YAML
policy_data              policy/*.toml
build_config             flake.nix, justfile
repo_config              clippy.toml, deny.toml, codecov.yml, etc.
generated_metadata       checked-in generator output
```

`covered_by` records the verification command(s) that prove the surface is
exercised — e.g. a test target, a `cargo run -p xtask -- ...` task, or an
external linter.

## Enforcement

```bash
cargo run -p xtask -- check-file-policy
```

Failure modes:

- **tracked or untracked non-ignored non-Rust file with no matching entry** —
  fail.
- **allowlist entry whose glob matches no tracked or untracked non-ignored
  file** — fail (unless `retired = true`).
- **allowlist entry past `expires`** — fail.
- **production / test / tooling entry without `covered_by`** — fail.

## Adding a new surface

1. Add the file or directory to the working tree.
2. Add an `[[allow]]` entry to `policy/non-rust-allowlist.toml`.
3. If the surface has executable, generated, dependency, workflow, process, or
   network behavior, add or update the matching companion ledger.
4. If a companion ledger uses a broad path glob, include
   `broad_glob_reason`.
5. Run `cargo run -p xtask -- check-file-policy` - it must pass.

## Removing a surface

If you delete a surface, also delete its allowlist entry. The checker fails
on stale entries so the policy stays honest.

## Relationship to Clippy and no-panic policy

Clippy and the semantic no-panic checker govern the **inside** of Rust
files. The file policy governs **which other files are allowed to exist**.
Together, they keep the repository's surface area small, owned, and
expiring.

## Companion ledgers

The non-Rust allowlist answers whether a non-Rust file may exist. Companion
ledgers answer what behavior those files may perform:

```text
policy/generated-allowlist.toml
policy/executable-allowlist.toml
policy/dependency-surface-allowlist.toml
policy/workflow-allowlist.toml
policy/process-allowlist.toml
policy/network-allowlist.toml
```

Each companion ledger uses schema `1.0`, requires at least one `[[allow]]`
entry, and each entry must include `id`, `owner`, `surface`, `behavior`,
`reason`, and non-empty `covered_by`. Generated entries also require
non-empty `generated_by`. The required locator depends on the ledger:
generated entries use `paths`; executable entries use `paths` or `commands`;
dependency entries use `paths` or `dependencies`; workflow entries use
`workflows`; process entries use `commands`; network entries use
`destinations`.

Use [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) for the map of current and
planned ledgers. The TOML files remain the source of truth.
