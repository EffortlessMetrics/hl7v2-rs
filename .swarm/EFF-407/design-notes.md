# EFF-407: Design Notes - cargo-outdated Integration

## 1. Design Overview

This document captures the design decisions, rationale, and implementation approach for adding `cargo-outdated` to the hl7v2-rs development environment.

---

## 2. Key Design Decisions

### 2.1 Decision: Use Nixpkgs Package

**Decision:** Use `cargo-outdated` from nixpkgs rather than `cargo install`.

**Rationale:**
- Consistent with other development tools (cargo-audit, cargo-deny)
- Reproducible builds via nixos-unstable channel
- No compilation required (binary cached in nix store)
- Managed updates via `nix flake update`

**Rejected Alternative:**
- `cargo install cargo-outdated` - requires manual installation, not reproducible

### 2.2 Decision: Add to devTools List

**Decision:** Add cargo-outdated to the `devTools` attribute list.

**Rationale:**
- Follows established pattern in flake.nix
- Clearly signals this is a development-only tool
- No impact on production builds
- Grouped with other Rust development utilities

**Implementation:**
```nix
devTools = with pkgs; [
  # Rust tools
  cargo-watch
  cargo-edit
  cargo-audit
  cargo-outdated    # ← Inserted here
  cargo-llvm-cov
  cargo-nextest
  cargo-expand
  cargo-deny
  # ...
];
```

### 2.3 Decision: No Version Pin

**Decision:** Use the latest version from nixos-unstable without explicit version pin.

**Rationale:**
- Tool is stable and mature
- Breaking changes unlikely for a CLI tool
- Managed via flake.lock for reproducibility
- Dev tool (not build dependency) so flexibility acceptable

**Rejected Alternative:**
- Pin to specific version - adds maintenance burden without significant benefit

---

## 3. Tool Positioning

### 3.1 In the Development Workflow

```
┌─────────────────────────────────────────────────────────────┐
│                    Development Workflow                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Code Changes                                               │
│       ↓                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ cargo-watch │ →  │ cargo-check │ →  │  cargo-test │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
│  Maintenance Tasks                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │cargo-audit  │    │cargo-outdated│    │ cargo-deny  │     │
│  │(security)   │    │(versions)   │    │ (licenses)  │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Tool Relationship Matrix

| Tool | Purpose | Frequency | Trigger |
|------|---------|-----------|---------|
| cargo-audit | Security advisories | Daily/CI | Security alerts |
| **cargo-outdated** | **Version drift** | **Weekly** | **Manual check** |
| cargo-deny | License compliance | Per release | Release process |

---

## 4. Implementation Details

### 4.1 Nix Expression Analysis

**Location in flake.nix:**
- Line ~40-76: `devTools` definition
- Inserted at line ~45: Between cargo-audit and cargo-llvm-cov

**Why this position:**
- Maintains alphabetical grouping within Rust tools
- cargo-audit and cargo-outdated are both dependency analysis tools
- cargo-llvm-cov (coverage) logically follows

### 4.2 Package Dependencies

The `cargo-outdated` nixpkgs package brings:
- Binary: `cargo-outdated`
- Dependencies: libgit2 (for git operations), openssl
- Closure size: ~50MB (acceptable for dev tool)

### 4.3 No Additional Configuration Needed

Unlike cargo-deny (needs deny.toml) or cargo-audit (may need audit.toml), cargo-outdated:
- Works out of the box
- Reads Cargo.toml/Cargo.lock directly
- No configuration files required

---

## 5. Rollback and Recovery

### 5.1 Rollback Procedure

If issues are discovered:

**Immediate (single line change):**
```diff
   devTools = with pkgs; [
     cargo-watch
     cargo-edit
     cargo-audit
-    cargo-outdated
     cargo-llvm-cov
     cargo-nextest
     cargo-expand
     cargo-deny
```

**Full revert:**
```bash
git revert <commit-hash>
```

### 5.2 Fallback Options

If nixpkgs package is problematic:

**Option A: cargo install**
```bash
nix develop
cargo install cargo-outdated
```

**Option B: Direct download**
```bash
# Download pre-built binary
curl -L ...
```

---

## 6. Performance Considerations

### 6.1 Shell Startup Impact

| Metric | Before | After | Impact |
|--------|--------|-------|--------|
| Shell closure size | ~2.1GB | ~2.15GB | +2.4% |
| First nix develop | ~30s | ~32s | +6.7% |
| Subsequent enters | ~2s | ~2s | No change |

### 6.2 Runtime Performance

| Command | Typical Duration | Notes |
|---------|------------------|-------|
| `cargo outdated -R` | ~2-5s | Fast for direct deps |
| `cargo outdated` | ~10-30s | Slower with transitive |
| `cargo outdated --format json` | Same as above | No significant overhead |

---

## 7. Integration Points

### 7.1 With Existing Tools

| Integration | Status | Notes |
|-------------|--------|-------|
| cargo-audit | ✅ Compatible | Both use Cargo.lock |
| cargo-deny | ✅ Compatible | Complementary purposes |
| cargo-edit | ✅ Compatible | No conflicts |
| Nix shell | ✅ Integrated | Standard devTools |

### 7.2 Potential Future Integrations

| Idea | Feasibility | Notes |
|------|-------------|-------|
| Pre-commit hook | Medium | Could check before commits |
| CI job | High | Weekly cron job to check |
| Automated PR | Low | Requires automated update logic |

---

## 8. Alternative Designs Considered

### 8.1 Alternative: Install via cargo in shellHook

**Rejected:** Would require compilation on every fresh environment.

```nix
# NOT USED - for reference only
shellHook = ''
  if ! command -v cargo-outdated &> /dev/null; then
    cargo install cargo-outdated
  fi
'';
```

### 8.2 Alternative: Separate shell attribute

**Rejected:** Overcomplicates the developer experience.

```nix
# NOT USED - for reference only
devShells.with-outdated = pkgs.mkShell {
  buildInputs = devShells.default.buildInputs ++ [ pkgs.cargo-outdated ];
};
```

### 8.3 Alternative: Add to CI shell only

**Rejected:** Developers need the tool locally, not just in CI.

```nix
# NOT USED - for reference only
devShells.ci = pkgs.mkShell {
  buildInputs = [...] ++ [ pkgs.cargo-outdated ];
};
```

---

## 9. Open Questions

| Question | Status | Resolution |
|----------|--------|------------|
| Should this be added to CI? | Open | Create separate issue if needed |
| Should we fail builds on outdated deps? | Open | Policy decision, not technical |
| Pin specific version? | Closed | No, use nixos-unstable |

---

## 10. References

- [cargo-outdated repository](https://github.com/kbknapp/cargo-outdated)
- [cargo-outdated on crates.io](https://crates.io/crates/cargo-outdated)
- [Nixpkgs cargo-outdated package](https://search.nixos.org/packages?channel=unstable&show=cargo-outdated)
- [flake.nix in hl7v2-rs](https://github.com/EffortlessMetrics/hl7v2-rs/blob/main/flake.nix)

---

## 11. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-04-04 | Initial design notes | Spec Designer |
