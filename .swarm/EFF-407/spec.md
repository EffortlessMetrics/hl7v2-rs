# EFF-407: cargo-outdated Installation - Execution Specification

## Overview

**Issue:** [EFF-407](/EFF/issues/EFF-407)  
**Title:** Install cargo-outdated for proactive dependency monitoring  
**Status:** Specification Phase  
**Branch:** `EFF-407-cargo-outdated`  
**Spec Designer:** Spec Designer

---

## 1. Problem Statement

The development environment lacked `cargo-outdated`, a critical tool for proactive dependency management. This created a gap where dependency drift could accumulate undetected until manual audits were performed.

### Evidence (from issue)
- ✅ cargo-audit (security scanning) - present
- ✅ cargo-deny (license/policy enforcement) - present  
- ❌ **cargo-outdated** (version drift detection) - **MISSING**

### Impact
1. No easy way to detect outdated dependencies during development
2. Dependency drift accumulates silently until issues emerge
3. Manual crate.io lookups required to check for updates
4. CI depends solely on dependabot which only runs weekly

---

## 2. Solution Summary

Add `cargo-outdated` to the Nix flake development shell's `devTools` list.

### Implementation Status
**COMPLETE** - The `cargo-outdated` package has been added to `flake.nix`:

```nix
devTools = with pkgs; [
  # Rust tools
  cargo-watch
  cargo-edit
  cargo-audit
  cargo-outdated    # ← ADDED
  cargo-llvm-cov
  cargo-nextest
  cargo-expand
  cargo-deny
  # ... other tools
];
```

---

## 3. Technical Specification

### 3.1 Nix Package Details

| Property | Value |
|----------|-------|
| Package Name | `cargo-outdated` |
| Nixpkgs Source | `nixos-unstable` |
| Package Function | Displays outdated Rust dependencies |
| Binary Name | `cargo-outdated` |

### 3.2 Nix Expression Location

**File:** `flake.nix`  
**Section:** `devTools` list within `devShells.default`  
**Line:** ~45 (within `devTools` attribute)

### 3.3 Tool Usage

After entering the Nix shell, developers can run:

```bash
# Show direct dependencies with newer versions available
cargo outdated -R

# Show all dependencies (including transitive)
cargo outdated

# Output as JSON for CI integration
cargo outdated --format json

# Check workspace dependencies
cargo outdated --workspace
```

### 3.4 Integration with Existing Tooling

The `cargo-outdated` tool complements the existing toolchain:

| Tool | Purpose | Integration |
|------|---------|-------------|
| cargo-audit | Security advisories | Parallel use |
| cargo-deny | License/policy enforcement | Parallel use |
| cargo-outdated | Version drift detection | **NEW** - fills the gap |

---

## 4. Requirements

### Functional Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| R1 | cargo-outdated available in Nix dev shell | ✅ Complete |
| R2 | Tool displays direct dependency updates (`-R` flag) | ✅ Available |
| R3 | Tool supports JSON output for CI integration | ✅ Available |
| R4 | Tool integrates with cargo workspace | ✅ Available |

### Non-Functional Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| NFR1 | No build time impact (dev tool only) | ✅ Met |
| NFR2 | No runtime dependencies added | ✅ Met |
| NFR3 | Consistent with existing tool installation pattern | ✅ Met |

---

## 5. BDD Scenarios

See detailed scenarios in: [bdd-scenarios.md](./bdd-scenarios.md)

### Summary

```gherkin
Feature: cargo-outdated integration in Nix development environment
  As a Rust developer
  I want to detect outdated dependencies quickly
  So that I can proactively manage dependency drift

  Scenario: Entering nix develop shell provides cargo-outdated
    Given the developer has cloned the repository
    When the developer runs "nix develop"
    Then cargo-outdated should be available in PATH
    And running "cargo outdated --version" should succeed

  Scenario: Detecting outdated direct dependencies
    Given the developer is in the nix develop shell
    When the developer runs "cargo outdated -R"
    Then the tool should list direct dependencies with available updates
    And the output should show current vs latest versions

  Scenario: CI integration via JSON output
    Given the CI pipeline has cargo-outdated installed
    When the pipeline runs "cargo outdated --format json -R"
    Then it should receive parseable JSON output
    And can fail the build if critical dependencies are outdated
```

---

## 6. Design Notes

### 6.1 Design Decisions

| Decision | Rationale |
|----------|-----------|
| Use Nixpkgs `cargo-outdated` | Consistent with other tools; reproducible builds |
| Add to `devTools` list | Follows existing pattern; dev-only, no build impact |
| No version pin | Uses nixos-unstable latest; acceptable for dev tool |

### 6.2 Options Considered

| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| A: Nixpkgs `cargo-outdated` | Reproducible, consistent with project | Requires Nix | ✅ Selected |
| B: `cargo install cargo-outdated` | Works without Nix | Not reproducible, manual step | ❌ Rejected |

### 6.3 Rollback Plan

If `cargo-outdated` causes issues:

1. **Immediate:** Remove from `devTools` list in `flake.nix`
2. **Alternative:** Use `cargo install cargo-outdated` as manual fallback
3. **Reversion:** `git revert` the change (single line removal)

---

## 7. Verification Steps

### 7.1 Manual Verification

```bash
# 1. Enter development shell
nix develop

# 2. Verify cargo-outdated is available
which cargo-outdated
cargo outdated --version

# 3. Check for outdated dependencies
cargo outdated -R

# 4. Test JSON output
cargo outdated --format json -R | jq .
```

### 7.2 Expected Results

- `cargo-outdated` binary should be in `$PATH`
- Version command should return valid version string
- Outdated check should complete without errors
- JSON output should be valid JSON

---

## 8. Artifacts Created

| Artifact | Location | Purpose |
|----------|----------|---------|
| This spec | `.swarm/EFF-407/spec.md` | Main specification document |
| BDD Scenarios | `.swarm/EFF-407/bdd-scenarios.md` | Test scenarios for downstream stages |
| Requirements | `.swarm/EFF-407/requirements.md` | Detailed requirements |
| Design Notes | `.swarm/EFF-407/design-notes.md` | Design decisions and rationale |

---

## 9. Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| cargo-outdated unavailable in nixos-unstable | Low | Medium | Use cargo install fallback |
| Version conflicts with cargo-audit | Low | Low | Both are mature, stable tools |
| Increased shell closure size | Low | Low | Single small binary |

---

## 10. Next Steps

1. **Spec Verifier** reviews this specification
2. Verify implementation matches spec
3. If changes needed, return to Code Builder
4. If spec approved, proceed to PR creation
5. Merge to main after tests pass

---

## 11. Appendix

### Related Issues
- Parent: [EFF-376](/EFF/issues/EFF-376) - Dependency Audit Summary
- Related: [EFF-406](/EFF/issues/EFF-406) - Dependency Drift Report
- Related: [EFF-246](/EFF/issues/EFF-246) - cargo-outdated (duplicate/related)

### References
- [cargo-outdated crate](https://crates.io/crates/cargo-outdated)
- [Nixpkgs cargo-outdated](https://search.nixos.org/packages?channel=unstable&show=cargo-outdated)
- [Nix flake documentation](https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html)
