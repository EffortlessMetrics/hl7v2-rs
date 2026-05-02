# Kyverno Policy Tests

This directory contains test fixtures for validating Kyverno security policies.

## Test Matrix

| Test | File | Expected | Description |
|------|------|----------|-------------|
| should-fail-latest | test-01-fail-latest.yaml | FAIL | Pod with `latest` tag |
| should-fail-no-digest | test-02-fail-no-digest.yaml | FAIL | Pod without digest |
| should-fail-always | test-03-fail-always.yaml | FAIL | Pod with `Always` pull policy |
| should-pass-valid | test-04-pass-valid.yaml | PASS | Pod with tag+digest, IfNotPresent |
| should-pass-deployment | test-05-pass-deployment.yaml | PASS | Deployment with valid config |
| should-exempt-kube-system | test-06-exempt-kube-system.yaml | SKIP | Pod in kube-system namespace |

## Running Tests

```bash
./validate-policies.sh
```

Or use kyverno CLI directly:

```bash
kyverno test ../require-image-digest-and-ifnotpresent.yaml ./
```

## Test File Format

Each test file is a standard Kubernetes manifest that can be applied to a cluster.
The tests validate the following policies:

1. **require-image-digest**: Containers must use images with SHA256 digests
2. **require-ifnotpresent**: Containers must use IfNotPresent pull policy (except kube-system)
3. **deny-latest-tag**: Containers cannot use `latest` tag
