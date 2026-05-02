# EFF-639: Container Image Vulnerability Scanning

## Overview
Add container image vulnerability scanning to the CI/CD pipeline using Trivy.

## Implementation Plan

### 1. Add container-scan job to `.github/workflows/security.yml`
- Build Docker image: `docker build -t hl7v2-rs:scan -f infrastructure/docker/Dockerfile .`
- Run Trivy scan with SARIF output
- Upload results to GitHub Security tab
- Block on CRITICAL/HIGH severity vulnerabilities

### 2. Add to nightly workflow
- Continuous monitoring of base image CVEs
- Drift detection for new vulnerabilities

### 3. Optional: SBOM generation
- Generate Software Bill of Materials for compliance

## References
- Issue: EFF-639
- Parent: EFF-1006 (Security: Kyverno Policy Compliance)
- Tool: [Trivy](https://github.com/aquasecurity/trivy-action)

## Test Plan
See `tests/container_vulnerability_scan_tests.rs` (to be added by Red Test Builder)

## Blockers
- OAuth scope limitation prevents direct workflow file modification via API
- Requires workflow scope to push `.github/workflows/security.yml` changes
