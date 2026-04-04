# EFF-407: BDD Scenarios - cargo-outdated Integration

## Feature: cargo-outdated in Nix Development Environment

### Background
```gherkin
Given the hl7v2-rs repository has been cloned
And the Nix package manager is installed
And the flake.nix includes cargo-outdated in devTools
```

---

## Scenario 1: cargo-outdated availability in nix develop shell

```gherkin
Scenario: Developer enters nix develop shell and has cargo-outdated
  Given the developer is in the repository root directory
  When the developer runs "nix develop"
  And the shell prompt appears
  Then the command "which cargo-outdated" should return a valid path
  And the command "cargo outdated --version" should exit with code 0
  And the version output should match pattern "cargo-outdated v\d+\.\d+\.\d+"
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| Clean repo | `nix develop` | Shell enters | Exit code 0 |
| In nix shell | `which cargo-outdated` | `/nix/store/.../bin/cargo-outdated` | Path exists |
| In nix shell | `cargo outdated --version` | `cargo-outdated v0.XX.X` | Regex match |

---

## Scenario 2: Detect outdated direct dependencies

```gherkin
Scenario: Developer checks for outdated direct dependencies
  Given the developer is in the nix develop shell
  And the Cargo.toml has dependencies defined
  When the developer runs "cargo outdated -R"
  Then the command should complete successfully
  And the output should display a table with columns: Name, Project, Compat, Latest, Kind, Platform
  Or the output should show "All dependencies are up to date, yay!" if no updates available
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| In nix shell | `cargo outdated -R` | Table or success message | Exit code 0 |
| Dependencies outdated | `cargo outdated -R` | Table with version columns | Parseable output |
| All up to date | `cargo outdated -R` | "yay!" message | String match |

---

## Scenario 3: JSON output for CI integration

```gherkin
Scenario: CI pipeline uses JSON output format
  Given the CI environment has cargo-outdated installed via nix
  When the pipeline runs "cargo outdated --format json -R"
  Then the output should be valid JSON
  And the JSON should contain an array of dependency objects
  And each object should have fields: name, project, latest, compat
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| In CI environment | `cargo outdated --format json -R` | Valid JSON | `jq .` succeeds |
| JSON output | Parse with jq | Array structure | `.[0].name` exists |
| JSON objects | Check fields | Has required fields | All fields present |

---

## Scenario 4: Workspace compatibility

```gherkin
Scenario: cargo-outdated works with workspace projects
  Given the developer is in a workspace root directory
  And the workspace has multiple crate members
  When the developer runs "cargo outdated --workspace -R"
  Then the command should check all workspace members
  And the output should include dependencies from all crates
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| Workspace root | `cargo outdated --workspace -R` | Aggregated results | All crates checked |
| Multi-crate workspace | Run command | Multiple crate sections | Section count >= 1 |

---

## Scenario 5: Integration with existing cargo tools

```gherkin
Scenario: cargo-outdated coexists with cargo-audit and cargo-deny
  Given the developer is in the nix develop shell
  When the developer runs "cargo audit --version"
  And the developer runs "cargo deny --version"
  And the developer runs "cargo outdated --version"
  Then all three commands should succeed
  And no tool should conflict with another
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| In nix shell | `cargo audit --version` | Version string | Exit code 0 |
| In nix shell | `cargo deny --version` | Version string | Exit code 0 |
| In nix shell | `cargo outdated --version` | Version string | Exit code 0 |
| All tools | Sequential run | All succeed | No conflicts |

---

## Scenario 6: No impact on build outputs

```gherkin
Scenario: cargo-outdated does not affect release builds
  Given the developer runs "nix build"
  When the build completes
  Then the resulting binary should not contain cargo-outdated code
  And the closure size should not include cargo-outdated
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| Clean repo | `nix build` | Successful build | Exit code 0 |
| Build output | Check runtime deps | No cargo-outdated | `ldd` or equivalent |
| Closure | Compare size | Similar to before | Within 1% of previous |

---

## Scenario 7: flake.nix structure preserved

```gherkin
Scenario: Adding cargo-outdated follows existing patterns
  Given the developer views flake.nix
  When examining the devTools list
  Then cargo-outdated should appear after cargo-audit
  And before cargo-llvm-cov
  And the alphabetical grouping should be maintained
```

**Behavior Grid:**

| Input | Action | Expected Output | Verification |
|-------|--------|-----------------|--------------|
| flake.nix | View devTools | Organized list | Pattern match |
| Tool list | Check ordering | Logical grouping | Alphabetical within groups |

---

## Test Implementation Notes

### Automated Testing Approach

1. **Unit test:** Verify flake.nix syntax with `nix flake check`
2. **Integration test:** Enter dev shell and run cargo-outdated commands
3. **E2E test:** Full workflow from `nix develop` to `cargo outdated -R`

### Manual Testing Checklist

- [ ] `nix develop` enters shell without errors
- [ ] `cargo outdated --version` returns version
- [ ] `cargo outdated -R` runs without errors
- [ ] `cargo outdated --format json` produces valid JSON
- [ ] Tool works alongside cargo-audit and cargo-deny
- [ ] No impact on `cargo build` or `cargo test`

### Expected Test Results

| Test | Expected Result |
|------|-----------------|
| Shell entry | Success |
| Version check | Returns semantic version |
| Outdated check | Success (may show empty or populated table) |
| JSON format | Valid parseable JSON |
| Parallel tool use | All succeed |
| Build | Unaffected |
