# Issues, Blockers, Next Steps, and Friction Points

## Overview
This document identifies current issues, blockers, next steps, and areas of friction in the hl7v2-rs project based on codebase analysis, documentation review, and implementation status.

## Current Issues and Blockers

### Test Coverage Gaps
Based on the STATUS.md file (last updated 2026-03-05), several crates have test coverage below 90%, indicating potential gaps in testing:

- **hl7v2-cli**: 75% coverage (lowest)
- **hl7v2-server**: 80% coverage
- **hl7v2-gen**: 80% coverage
- **hl7v2-validation**: 82% coverage
- **hl7v2-prof**: 85% coverage
- **hl7v2-query**: 90% coverage
- **hl7v2-stream**: 88% coverage
- **hl7v2-writer**: 94% coverage
- **hl7v2-parser**: 95% coverage
- **hl7v2-core**: 92% coverage

**Impact**: Lower test coverage increases the risk of undetected bugs and reduces confidence in making changes to these components.

### Documentation Debt
Several documentation files serve only as redirects:
- `IMPLEMENTATION_PLAN.md`: States it has been superseded by ROADMAP.md
- `IMPLEMENTATION_STATUS.md`: States it has moved to docs/STATUS.md

While not critical, this creates minor friction for users seeking information.

### v1.3.0 Features Not Yet Implemented
The ROADMAP.md indicates v1.3.0 is in the planning phase with the following features not yet started:
- gRPC Support
- Language Bindings (Python/Java)
- WASM Target
- Custom Field Rules

These represent significant upcoming work that hasn't begun implementation.

## Next Steps (Based on SESSION_SUMMARY.md)

### Immediate Priorities (Current Sprint)
According to the SESSION_SUMMARY.md file, the team has outlined an 8-week sprint plan:

**Weeks 1-2 (Now)**: Network Module Foundation
- MLLP codec implementation
- TCP server foundation

**Weeks 3-4 (Next)**: Backpressure & Memory Bounds
- Implementation of proper channel-based flow control
- Configurable memory limits to prevent DoS

**Weeks 5-6 (Later)**: HTTP Server with Axum
- Production-ready REST API implementation
- OpenAPI/Swagger UI integration

**Week 7**: Property-Based Testing
- Implementation of property-based testing for edge cases

**Week 8**: Documentation & Examples
- Comprehensive documentation updates
- Working examples for all features

## Friction Points and Areas for Improvement

### 1. Test Coverage Improvement
The varying test coverage percentages suggest an opportunity to:
- Establish minimum test coverage thresholds (e.g., 90% for all crates)
- Focus testing efforts on lower-coverage crates (CLI, server, gen)
- Consider property-based testing for complex validation logic

### 2. Documentation Consistency
While the ADR system is well-established, some older documents duplicate information:
- Consider consolidating planning documents
- Ensure all crate-specific READMEs and CLAUDE files are complete and up-to-date

### 3. v1.3.0 Preparation
Although v1.3.0 is still in planning, preliminary work could include:
- Creating spike solutions for gRPC integration with Tonic
- Researching WebAssembly target options for Rust
- Defining initial language binding interfaces
- Designing the custom field rules extension point

### 4. Performance Benchmarks
The repository has a `hl7v2-bench` crate, but it would be beneficial to:
- Establish baseline performance benchmarks for v1.2.0
- Create performance regression tests
- Document performance characteristics in the README or STATUS.md

### 5. Developer Onboarding
While CONTRIBUTING.md exists, consider:
- Creating a "Getting Started" guide for new contributors
- Adding more detailed examples in the examples/ directory
- Creating tutorial-style documentation for common use cases

## Recommendations

### Short-term (Current Sprint)
1. Focus on completing the Network Module Foundation (weeks 1-2)
2. Begin implementing backpressure mechanisms (weeks 3-4)
3. Address lowest test coverage areas (CLI first, then server/gen)

### Medium-term (v1.3.0 Preparation)
1. Create technical spikes for gRPC/WASM/Language bindings
2. Define clear interfaces for custom field rules
3. Begin drafting ADRs for v1.3.0 architectural decisions
4. Establish contribution guidelines for new feature areas

### Long-term (v2.0.0 Vision)
1. Monitor community feedback on v1.2.0 for HIPAA readiness requirements
2. Research cloud storage integration patterns
3. Explore analytics dashboard technologies
4. Begin database integration prototypes

## Conclusion

The hl7v2-rs project has successfully achieved v1.2.0 production readiness with strong foundational work completed. The primary focus should now be on executing the defined sprint plan to solidify the network and HTTP server capabilities, while beginning to lay the groundwork for v1.3.0 enterprise features.

Addressing the test coverage gaps, particularly in the CLI and server components, will increase confidence in the codebase as it grows in complexity. The established ADR process provides an excellent mechanism for documenting architectural decisions as the project evolves toward v1.3.0 and beyond.
