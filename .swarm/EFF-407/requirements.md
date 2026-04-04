# EFF-407: Requirements - cargo-outdated Integration

## 1. Functional Requirements

### FR-1: Package Availability
**Requirement:** The `cargo-outdated` binary must be available in the Nix development shell.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Running `nix develop` provides access to `cargo-outdated` command
- `which cargo-outdated` returns a valid path in the nix store
- The tool is in the shell's `$PATH`

### FR-2: Direct Dependency Checking
**Requirement:** Developers must be able to check for outdated direct dependencies.  
**Priority:** Must Have  
**Acceptance Criteria:**
- `cargo outdated -R` command executes successfully
- Output displays current project version vs. latest available version
- Works for all workspace crate dependencies

### FR-3: JSON Output Format
**Requirement:** The tool must support JSON output for programmatic use.  
**Priority:** Should Have  
**Acceptance Criteria:**
- `cargo outdated --format json` produces valid JSON output
- JSON structure includes: name, project, compat, latest fields
- Output can be piped to tools like `jq`

### FR-4: Workspace Support
**Requirement:** The tool must work with Cargo workspace projects.  
**Priority:** Must Have  
**Acceptance Criteria:**
- `cargo outdated --workspace` checks all workspace members
- Results aggregate across the entire workspace
- Works from workspace root directory

### FR-5: Compatibility Mode
**Requirement:** Tool should show semver-compatible updates separately from latest.  
**Priority:** Should Have  
**Acceptance Criteria:**
- Compat column shows latest semver-compatible version
- Latest column shows absolute latest version
- Developers can distinguish between safe and breaking updates

## 2. Non-Functional Requirements

### NFR-1: Build Isolation
**Requirement:** cargo-outdated must not affect release builds or runtime.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Tool is only in dev shell, not in build inputs
- Release builds do not include cargo-outdated
- No runtime dependencies on cargo-outdated

### NFR-2: Tool Consistency
**Requirement:** Installation must follow existing tool patterns in flake.nix.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Added to `devTools` list alongside cargo-audit, cargo-deny
- Uses same `with pkgs;` import pattern
- Grouped with other Rust development tools

### NFR-3: No Version Conflicts
**Requirement:** cargo-outdated must not conflict with other cargo subcommands.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Can run alongside cargo-audit without issues
- Can run alongside cargo-deny without issues
- No shared dependency conflicts

### NFR-4: Reproducibility
**Requirement:** Tool installation must be reproducible via Nix.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Uses nixpkgs package (not cargo install)
- Locked to nixos-unstable channel via flake.lock
- Same version available across different machines

### NFR-5: Shell Startup Performance
**Requirement:** Adding cargo-outdated must not significantly impact shell startup.  
**Priority:** Should Have  
**Acceptance Criteria:**
- `nix develop` startup time increase < 10%
- No additional nix evaluation overhead beyond package reference

## 3. Integration Requirements

### IR-1: Toolchain Coexistence
**Requirement:** cargo-outdated must integrate with existing Rust toolchain.  
**Priority:** Must Have  
**Acceptance Criteria:**
- Works with rust-overlay provided Rust toolchain
- Compatible with cargo-watch workflow
- Does not interfere with cargo-edit

### IR-2: CI Compatibility
**Requirement:** Tool must work in CI environments using the Nix flake.  
**Priority:** Should Have  
**Acceptance Criteria:**
- `nix develop -c cargo outdated` works in CI
- Can fail builds based on outdated dependencies
- JSON output parseable by CI scripts

## 4. Documentation Requirements

### DR-1: Developer Documentation
**Requirement:** Developers must know how to use cargo-outdated.  
**Priority:** Should Have  
**Acceptance Criteria:**
- DEVELOPMENT.md mentions cargo-outdated
- Basic usage examples provided
- Cross-reference with cargo-audit and cargo-deny

### DR-2: Spec Documentation
**Requirement:** Implementation must be documented in .swarm/.  
**Priority:** Must Have  
**Acceptance Criteria:**
- spec.md created in `.swarm/EFF-407/`
- bdd-scenarios.md created
- design decisions documented

## 5. Rollback Requirements

### RR-1: Easy Removal
**Requirement:** Tool must be easily removable if issues arise.  
**Priority:** Should Have  
**Acceptance Criteria:**
- Single line removal from flake.nix
- No cascading changes required
- Can be reverted with `git revert`

## 6. Verification Matrix

| Requirement | Test Method | Pass Criteria | Status |
|-------------|-------------|---------------|--------|
| FR-1 | Manual test | `which cargo-outdated` returns path | ⏳ Pending |
| FR-2 | Manual test | `cargo outdated -R` succeeds | ⏳ Pending |
| FR-3 | Script test | JSON output parses with jq | ⏳ Pending |
| FR-4 | Manual test | Works in workspace root | ⏳ Pending |
| NFR-1 | Build test | Release build excludes tool | ⏳ Pending |
| NFR-2 | Review | Follows existing pattern | ✅ Verified |
| NFR-3 | Integration test | No conflicts with audit/deny | ⏳ Pending |
| NFR-4 | Flake test | Reproducible across machines | ⏳ Pending |
