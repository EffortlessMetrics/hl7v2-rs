# Contributing to hl7v2-rs

Thank you for considering contributing to hl7v2-rs! This project is actively developing v1.2.0 with significant work on server mode, remote profiles, and streaming improvements. We welcome contributions and have organized this guide to help you get started.

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions. We're building healthcare infrastructure—let's do it right.

---

## Quick Start for Contributors

### I want to...

- **Report a bug**: [Open a bug report](#reporting-bugs)
- **Request a feature**: [Open a feature request](#requesting-features)
- **Contribute code**: [Follow the development workflow](#development-workflow)
- **Improve documentation**: [Edit docs](#documentation)
- **Help with testing**: [Write or improve tests](#testing)

---

## Development Workflow

### 1. Set Up Your Environment

Follow [DEVELOPMENT.md](DEVELOPMENT.md) for detailed setup instructions.

```bash
# Quick start
git clone https://github.com/EffortlessMetrics/hl7v2-rs.git
cd hl7v2-rs
cargo build
cargo test
```

### 2. Understand the Project Status

**Start here**: [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)
- What's implemented
- What's being worked on
- Known limitations

**Then read**: [ROADMAP.md](ROADMAP.md)
- v1.2.0 goals (next 8 weeks)
- Feature priorities
- Dependencies and critical path

**Finally**: [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)
- Sprint-level tasks
- Story point estimates
- Detailed technical requirements

### 3. Pick an Issue

**For v1.2.0 work** (active development):
- Search GitHub Issues for `[v1.2.0]` label
- Check IMPLEMENTATION_PLAN.md for sprint tasks
- Look for `good-first-issue` if you're new

**Areas that need help**:
- **Network module** (CRITICAL PRIORITY)
- Server mode HTTP/gRPC endpoints
- Remote profile loading
- CLI enhancements
- Testing and benchmarks

### 4. Create a Feature Branch

```bash
# Branch naming: <type>/<description>
# Types: feature, fix, docs, test, refactor

git checkout -b feature/streaming-backpressure
git checkout -b fix/profile-cycle-detection
git checkout -b docs/testing-guide
```

### 5. Implement Your Changes

**Code Style**:
- Use `cargo fmt` for formatting (enforced in CI)
- Follow `cargo clippy` recommendations
- Run `cargo clippy --all` before pushing
- Aim for zero warnings

**Testing**:
- Write tests alongside your code (TDD preferred)
- Aim for 90%+ coverage of new code
- Add integration tests for new features
- See [TESTING.md](TESTING.md) for procedures

**Documentation**:
- Document public APIs with examples
- Add/update README sections if needed
- Comment complex algorithms
- Update IMPLEMENTATION_STATUS.md if changing features

### 6. Commit Your Work

**Commit Message Format**:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: feat, fix, docs, style, refactor, test, chore
**Scope**: core, prof, gen, cli, network, etc.
**Subject**: 50 chars max, present tense, no period

**Examples**:
```
feat(core): add bounded event queue for streaming parser

Implement BoundedEventQueue with configurable capacity to enforce
backpressure on streaming parser. Adds --queue-capacity CLI flag
with default 1024 messages.

Closes #42
```

```
fix(prof): detect circular profile inheritance chains

Add cycle detection in load_profile_with_inheritance() to prevent
infinite loops. Returns E_Profile_Cycle error with chain details.

Fixes #38
```

### 7. Create a Pull Request

**Title Format**:
```
[v1.2.0] <Feature/Fix Description>
```

**PR Description** (use template in .github/):
- Briefly explain what/why
- Link to related issues
- Describe testing approach
- Note any breaking changes

**Reviewers**:
- Tag relevant team members
- Allow 1-3 business days for review
- Be open to feedback

### 8. Address Review Comments

- Reply to each comment
- Update code if needed
- Force-push updates (don't add merge commits)
- Re-request review when done

### 9. Merge & Deploy

- Require 2 approvals for core changes
- Pass all CI checks
- Squash commit if multiple small commits
- Delete branch after merge

---

## Reporting Bugs

### Before You Report

- Check [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) - it's a known limitation?
- Search existing issues
- Try on latest main branch

### How to Report

[Use the Bug Report template](.github/ISSUE_TEMPLATE/bug_report.md)

Include:
- Clear description
- Minimal reproduction
- Expected vs actual behavior
- Environment (OS, Rust version, etc.)
- Logs/errors (if any)

**Example**:
```markdown
**Describe the bug**
Profile validation crashes on circular inheritance.

**To Reproduce**
1. Create profile_a.yaml with `parent: profile_b`
2. Create profile_b.yaml with `parent: profile_a`
3. Run `hl7v2 val message.hl7 --profile profile_a.yaml`

**Expected**
Should return E_Profile_Cycle error with chain details.

**Actual**
Stack overflow / panic

**Environment**
- OS: Linux
- Rust: 1.89
```

---

## Requesting Features

### Before You Request

- Check ROADMAP.md - is it already planned?
- Check IMPLEMENTATION_PLAN.md - is it in sprint work?
- Check existing issues

### How to Request

[Use the Feature Request template](.github/ISSUE_TEMPLATE/feature_request.md)

Include:
- Use case / problem you're solving
- Proposed solution
- Alternatives you've considered
- Is this for v1.2, v1.3, or v2.0?

**Example**:
```markdown
**Use Case**
Currently no way to cache remote profiles, causing repeated HTTP calls.

**Proposed Solution**
Add LRU cache with ETag support to profile loader as per ROADMAP.md v1.2.0.

**Alternatives**
- Let users implement caching in wrapper code (not ideal)
- Simple file-based cache (insufficient for production)

**Timeline**
This is in IMPLEMENTATION_PLAN.md Sprint 4, so high priority for v1.2.0.
```

---

## Documentation

### Documentation Style

- **README.md**: Features, quick start, high-level architecture
- **IMPLEMENTATION_STATUS.md**: What's actually implemented now
- **ROADMAP.md**: Future direction and timelines
- **DEVELOPMENT.md**: Developer setup and workflow
- **Design docs** (.qoder/quests/): Detailed technical specifications
- **Code comments**: Explain the "why", not the "what"
- **Rustdoc**: Public API documentation with examples

### How to Contribute Docs

1. Check if content should live in docs/ or codebase
2. Follow the style of existing docs
3. Include working examples
4. Test links (especially cross-file references)
5. Submit PR with `docs()` commit type

**Examples of good doc contributions**:
- Expanded CLI usage examples
- Tutorial for building a validator
- Deployment guide
- Architecture diagram
- Performance tuning guide

---

## Testing

### Running Tests Locally

```bash
# Unit tests
cargo test --all

# Integration tests
cargo test --all --test '*'

# Benchmarks
cargo bench

# Coverage (with tarpaulin)
cargo tarpaulin --all
```

See [TESTING.md](TESTING.md) for detailed procedures.

### Test Coverage

- Target: 90%+ for new code
- Core functionality: 95%+
- All public APIs must have examples/tests

### Writing Tests

**Location**: Tests go in:
- `src/tests.rs` (unit tests, same file as code)
- `tests/` directory (integration tests)
- Or use `#[cfg(test)]` modules

**Pattern**:
```rust
#[test]
fn test_feature_happy_path() {
    let input = /* ... */;
    let result = my_function(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_feature_error_case() {
    let input = /* invalid input */;
    let result = my_function(input);
    assert!(result.is_err());
}
```

---

## Performance & Benchmarking

### Performance Targets (v1.2.0)

See [ROADMAP.md](ROADMAP.md) for complete list:
- Parse: ≥100k small msgs/min
- Server: ≥1000 RPS sustained
- Memory: Proportional to message size
- Latency: <5ms p95 (typical message)

### Benchmarking Your Code

```bash
# Run benchmarks
cargo bench

# Benchmark specific function
cargo bench -- --exact my_function_name

# Save baseline
cargo bench -- --save-baseline before_optimization
cargo bench -- --baseline before_optimization
```

See [TESTING.md](TESTING.md) for profiling tools.

---

## v1.2.0 Priorities

From [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md), these are critical:

1. **Network Module** (BLOCKING)
   - MLLP frame handler
   - TCP connection management
   - Tests

2. **Server Mode HTTP**
   - /hl7/parse endpoint
   - /hl7/validate endpoint
   - /hl7/ack endpoint
   - Error handling

3. **Remote Profile Loading**
   - HTTP fetching
   - S3/GCS support
   - LRU caching

4. **Authentication/RBAC**
   - Bearer token validation
   - Role-based access control
   - PHI redaction in logs

5. **Streaming Improvements**
   - Backpressure with bounded queue
   - Memory bounds enforcement
   - Resume across boundaries

---

## Getting Help

- **Questions about contributing?** Open a GitHub discussion
- **Need clarification on a task?** Comment on the related issue
- **Want to pair program?** Reach out to team members
- **Found a blocker?** Post in Discord/Slack (team channel)

---

## Review Process

### What Reviewers Look For

✅ **Good Reviews Check**:
- Code follows style guide (fmt + clippy)
- Tests cover new functionality
- Docs are updated
- No performance regressions
- Error handling is robust
- Security implications considered

❌ **Common Rejection Reasons**:
- Missing tests
- Clippy warnings
- Undocumented public APIs
- Performance degradation
- Breaking changes without migration path

### Making Reviews Easier

- Keep PRs focused (one feature per PR)
- Write clear commit messages
- Include context in PR description
- Reference related issues
- Respond promptly to feedback

---

## Dependency Policy

### Adding Dependencies

- Prefer existing dependencies
- Check for security vulnerabilities (`cargo audit`)
- Consider compile-time impact
- Avoid heavy transitive deps
- Prefer `dev-dependencies` for testing

### Dependency Updates

- Regular audits (`cargo audit`)
- Keep MSRV (Minimum Supported Rust Version) in mind
- Test compatibility before upgrading

---

## Releases

The team follows [semantic versioning](https://semver.org/):
- **PATCH** (1.1.x): Bug fixes, non-breaking
- **MINOR** (1.2.0): New features, backward compatible
- **MAJOR** (2.0.0): Breaking changes

**Release Process**:
1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Tag release: `v1.2.0`
4. Push to crates.io
5. Create GitHub release

Contributors don't need to handle releases; maintainers do.

---

## Recognition

Contributors are recognized in:
- CHANGELOG.md (all changes)
- GitHub contributors page (automatic)
- Release notes (major contributors)

We value all contributions—code, docs, testing, design discussion, etc.

---

## Questions?

- **Code questions**: Open GitHub discussions
- **Process questions**: Comment on issues/PRs
- **General**: Check [ROADMAP.md](ROADMAP.md) and [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) first

---

## Code of Conduct

We're committed to providing a welcoming and inclusive environment:
- Respect different perspectives
- Assume good intent
- Be constructive in feedback
- Report violations to maintainers

---

**Thank you for contributing to hl7v2-rs!**

Your work helps bring robust, open-source HL7v2 processing to the healthcare community.
