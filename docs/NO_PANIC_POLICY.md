# No-Panic Policy

`hl7v2-rs` treats panic-family behavior as a governed engineering surface.
Production code, library code, and tests are expected to be panic-free by
default. Any unavoidable exception must be a narrow, owned, expiring,
reviewable receipt, not a silent suppression.

This policy operates as a dual rail:

- Rail A, Clippy, detects panic-family code shapes locally and in CI via the
  workspace `[lints.clippy]` baseline.
- Rail B, the semantic no-panic checker, owns the authoritative exception
  ledger via `policy/no-panic-allowlist.toml` and is enforced by
  `cargo run -p xtask -- check-no-panic-family`.

Clippy is the fast feedback loop. The semantic checker carries the receipt
metadata (path, family, selector, owner, classification, reason, expiry, and
last seen) that an `#[expect(...)]` attribute alone cannot.

## Panic families

The checker tracks the following call shapes:

| family | shape |
| --- | --- |
| `unwrap` | `EXPR.unwrap()` |
| `expect` | `EXPR.expect(MSG)` |
| `get_unwrap` | `EXPR.get_unwrap(IDX)` |
| `panic_macro` | `panic!(...)` |
| `todo` | `todo!(...)` |
| `unimplemented` | `unimplemented!(...)` |
| `unreachable` | `unreachable!(...)` |
| `indexing` | `EXPR[IDX]` on slice/Vec/array |
| `string_slice` | `STR[a..b]` on string slices |

Assertion macros (`assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`) are
intentionally out of scope for v1. Tests still use them as oracles. Migrating
to fallible assertion helpers is a later, separately tracked decision.

## Identity

The checker matches findings to allowlist entries by:

```text
identity = path + family + selector_kind + selector_callee + snippet + count
```

`count` is consumed per occurrence. A receipt for one `unwrap` does not cover a
second matching call unless its `count` explicitly says so. Duplicate allowlist
identities are rejected.

The `snippet` is the stripped source line containing the finding. Strings and
comments are removed before the snippet is captured, so receipts stay focused
on code shape rather than message text.

`container`, `last_seen.line`, and `last_seen.column` are advisory locators only
and are never part of the match key.

The selector is structural:

```toml
[allow.selector]
kind = "method_call"          # method_call | macro | indexing
container = "parses_msh"      # enclosing `fn` name (best-effort locator)
callee = "unwrap"             # method or macro name
```

Matching consumes exact allowlist count slots first. The no-new-debt baseline
consumes baseline count slots after allowlist slots.

## Allowlist schema

`policy/no-panic-allowlist.toml` uses schema `0.4`:

```toml
schema_version = "0.4"

[[allow]]
id = "panic-0001"
path = "crates/example/src/parser.rs"
family = "unwrap"
snippet = "let parsed = parse(sample).unwrap();"
count = 1
classification = "test_helper"
owner = "parser"
explanation = "Fixture builder; migrate to fallible helper."
expires = "2026-07-01"

[allow.selector]
kind = "method_call"
container = "parses_boundary_fixture"
callee = "unwrap"

[allow.last_seen]
line = 42
column = 17
```

Required fields per entry: `id`, `path`, `family`, `snippet`, `count`,
`classification`, `owner`, `explanation`, `expires`, `[allow.selector]` with
`kind` and `callee`. `container` and `[allow.last_seen]` are optional but
recommended locators.

`classification` is one of:

```text
production   - intentional in a non-test surface (must be rare)
test_helper  - fixture/setup code in tests
generated    - emitted by codegen we do not own
fixture      - test data harness
external_api - boundary with crate where unwrap is documented infallible
```

## Scope

The checker scans Rust source under crates that inherit the workspace
panic-family baseline, plus `xtask/`. The set of inheriting crates is sourced
from `policy/clippy-lints.toml`'s `rollout.required_inheriting_packages` and
`rollout.staged_inheriting_packages`. Staged packages are validated in
report-only mode (no failure) until they flip to required.

Doc comments and macro-internal example code are skipped: they compile through
doctests and are governed by Clippy directly.

## Enforcement

```bash
# Strict check (CI parity): fail on unallowlisted, stale, or expired entries.
cargo run -p xtask -- check-no-panic-family

# Propose new allowlist entries based on current findings.
cargo run -p xtask -- no-panic propose

# Refresh the generated no-new-debt baseline.
cargo run -p xtask -- no-panic baseline

# Regenerate the baseline from the current tree. Use only in the dedicated
# baseline PR.
cargo run -p xtask -- no-panic baseline --reset

# Combined report alongside clippy lint policy state.
cargo run -p xtask -- policy-report
```

Failure modes:

- finding without allowlist entry: fail.
- finding count greater than available allowlist count: fail.
- allowlist entry not fully matched by findings: fail as stale/surplus receipt.
- allowlist entry past `expires`: fail.
- drift in `container` or `[allow.last_seen]`: advisory only; rerun
  `no-panic propose` to refresh locators.

`policy/no-panic-baseline.toml` is generated and marked as generated in
`.gitattributes`. Normal refreshes may drop disappeared entries or reduce
counts, but they refuse to absorb new debt. `--reset` may absorb all current
findings and is only valid in the dedicated baseline PR.

`no-panic propose` emits a candidate TOML at
`target/policy/no-panic-proposed-allowlist.toml`. It never mutates
`policy/no-panic-allowlist.toml` automatically. A reviewer copies the proposed
entries in and adds owner, classification, explanation, and expiry before
merging.

## Adding an exception

1. Write the code with the unavoidable panic-family form.
2. Add a narrow `#[expect(clippy::<lint>, reason = "policy:no-panic:<id>")]`
   on the smallest enclosing item if Clippy still fires.
3. Add an `[[allow]]` entry to `policy/no-panic-allowlist.toml` with exact
   `snippet`, `count`, owner, classification, explanation, and `expires` (90
   days or less unless explicitly justified).
4. Run `cargo run -p xtask -- check-no-panic-family`; it must pass.
5. Open the PR. The receipt is reviewable; `expires` puts the cleanup on a
   clock.

## Burn-down

Drop receipts when the underlying code is migrated. The checker fails on stale
or surplus entries, so cleanup is forced at the next CI run.
