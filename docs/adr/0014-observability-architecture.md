# ADR-008: Observability Architecture

**Date**: 2026-03-04
**Status**: ACCEPTED
**Author**: Gemini CLI
**Deciders**: Project Team

## Context

Production-ready healthcare systems require deep visibility into their internal state. For `hl7v2-rs`, we need to monitor throughput, error rates, and latency across both the HTTP and MLLP interfaces.

### Requirements

- Real-time monitoring of request volume and success/failure rates.
- Latency tracking for performance SLA verification.
- Visibility into HL7-specific errors (validation failures vs. parse errors).
- Integration with industry-standard tooling (Prometheus, Grafana).
- Structured logging for correlation and troubleshooting.

## Decision

We will adopt a **Prometheus-first** metrics strategy combined with **Structured Tracing** for logging.

### 1. Metrics (Prometheus)

We use `metrics` and `metrics-exporter-prometheus` to expose a `/metrics` endpoint.

**Core Metrics Categories:**
- **HTTP Metrics**: `hl7v2_requests_total` (labels: method, path, status), `hl7v2_request_duration_seconds`.
- **HL7 Logic Metrics**: `hl7v2_parse_errors_total`, `hl7v2_validation_errors_total`.
- **System Metrics**: `hl7v2_active_connections`, `hl7v2_uptime_seconds`.
- **Resource Metrics**: `hl7v2_message_size_bytes`.

### 2. Logging (Tracing)

We use the `tracing` crate for all logging.

- **Format**: JSON in production (via `tracing-subscriber`), human-readable in development.
- **Levels**: `INFO` for lifecycle events, `DEBUG` for request/response metadata, `TRACE` for segment-level details.
- **Context**: Every request includes a unique `request_id` for log correlation.

### 3. Health & Readiness

Separate endpoints are provided to distinguish between "process is alive" and "process is ready to handle HL7 traffic".

- `/health`: Liveness probe. Returns 200 if the process is running.
- `/ready`: Readiness probe. Returns 200 only if all dependencies (config, local FS) are loaded.

## Rationale

1.  **Standardization**: Prometheus is the de-facto standard for cloud-native monitoring.
2.  **Performance**: The `metrics` crate uses atomic counters and efficient buffers to minimize impact on the request hot path.
3.  **Correlation**: Structured logging allows DevOps teams to use tools like ELK or Loki to filter by `request_id` or `patient_id` (if anonymized) across multiple service hops.

## Consequences

### Positive

- ✅ Full visibility into production performance.
- ✅ Easy alerting for "High Validation Failure Rate" (potential upstream interface change).
- ✅ Seamless integration with Kubernetes HPA (Horizontal Pod Autoscaler).

### Negative

- ⚠️ Slightly increased binary size due to tracing/metrics dependencies.
- ⚠️ Internal metrics state consumes a small, fixed amount of memory.

## References

- [ADR-006: Rate Limiting and Backpressure Strategy](ADR-006-rate-limiting-and-backpressure.md)
- [Prometheus Documentation](https://prometheus.io/docs/introduction/overview/)
- [Rust Tracing Ecosystem](https://tracing.rs/tracing/)
