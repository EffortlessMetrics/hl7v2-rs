# Multi-stage Dockerfile for the standalone HL7v2 validation sidecar.

FROM rust:1.93-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release --locked -p hl7v2-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 hl7v2 \
    && useradd --uid 1000 --gid hl7v2 --create-home --shell /usr/sbin/nologin hl7v2 \
    && mkdir -p /etc/hl7v2 /var/lib/hl7v2/bundles /var/lib/hl7v2/quarantine /var/lib/hl7v2/profiles \
    && chown -R hl7v2:hl7v2 /etc/hl7v2 /var/lib/hl7v2

COPY --from=builder /app/target/release/hl7v2-server /usr/local/bin/hl7v2-server

USER hl7v2

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:8080/ready >/dev/null || exit 1

ENTRYPOINT ["hl7v2-server"]
