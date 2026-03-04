# ADR-007: Authentication Strategy

**Date**: 2026-03-04
**Status**: ACCEPTED
**Author**: Gemini CLI
**Deciders**: Project Team

## Context

The `hl7v2-server` provides HTTP/REST endpoints for parsing and validating HL7v2 messages. As these messages often contain Protected Health Information (PHI), securing access to the API is critical. We need an authentication strategy that balances security, simplicity for internal integration, and a clear path toward more robust standards.

### Requirements

- Prevent unauthorized access to HL7 processing endpoints.
- Support common integration patterns for healthcare IT systems.
- Easy to rotate and manage in CI/CD and Kubernetes environments.
- Minimal latency overhead for high-throughput processing.
- Fail-closed behavior (no access if misconfigured).

## Decision

We will implement **API Key Authentication** via a dedicated middleware for the v1.2.0 release.

### Implementation Details

1.  **Transport Security**: The server assumes it is running behind a TLS-terminating reverse proxy (e.g., Nginx, Traefik, or a Cloud Load Balancer). Authentication is only considered secure over HTTPS.
2.  **Header-based**: Clients must provide the key in the `X-API-Key` HTTP header.
3.  **Environment Configuration**: The expected key is loaded from the `HL7V2_API_KEY` environment variable.
4.  **Fail-Closed**: If `HL7V2_API_KEY` is not set or is empty, the server will log an error and return `500 Internal Server Error` for all authenticated routes, preventing accidental open access.
5.  **Middleware Integration**: The authentication layer is applied to all `/hl7/*` routes while keeping `/health`, `/ready`, and `/metrics` (optionally) public.

### Example Usage

```bash
curl -X POST http://localhost:8080/hl7/parse \
  -H "X-API-Key: your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{"message":"..."}'
```

## Rationale

1.  **Simplicity**: API keys are the industry standard for server-to-server integration where OIDC/OAuth2 might be overkill for internal service mesh traffic.
2.  **Performance**: Validating a static string in a header has near-zero latency impact compared to JWT validation or OIDC introspection.
3.  **Kubernetes Native**: API keys map perfectly to Kubernetes Secrets and are easily injected as environment variables.
4.  **Standard for MVP**: This provides immediate "Production Readiness" for v1.2.0 while acknowledging that more complex requirements (RBAC, Scopes) will come in v2.0.0.

## Consequences

### Positive

- ✅ Immediate protection against unauthorized network access.
- ✅ Easy integration for legacy systems and simple scripts.
- ✅ Zero external dependencies (no need for an Identity Provider like Keycloak for the base install).

### Negative

- ⚠️ Keys are static and must be rotated manually or via orchestration.
- ⚠️ No built-in support for multiple keys or granular permissions (all or nothing).
- ⚠️ Vulnerable if the transport layer (HTTPS) is not properly configured.

## Future Evolution

Planned for v2.0.0:
- Support for multiple valid API keys with rotation windows.
- Optional OAuth 2.0 / OIDC integration for organizations with centralized IAM.
- mTLS (mutual TLS) support for high-security environments.

## References

- [ADR-006: Rate Limiting and Backpressure Strategy](ADR-006-rate-limiting-and-backpressure.md)
- [Axum Middleware Documentation](https://docs.rs/axum/latest/axum/middleware/index.html)
- [HIPAA Security Rule: Technical Safeguards](https://www.hhs.gov/hipaa/for-professionals/security/guidance/index.html)
