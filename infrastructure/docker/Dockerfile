# Multi-stage Dockerfile for optimized hl7v2-rs builds

# Stage 1: Build
FROM rust:1.89-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build dependencies (cached layer)
RUN cargo build --release --locked

# Stage 2: Runtime
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    openssl

# Create non-root user
RUN addgroup -g 1000 hl7v2 && \
    adduser -D -u 1000 -G hl7v2 hl7v2

# Copy binary from builder
COPY --from=builder /app/target/release/hl7v2 /usr/local/bin/hl7v2

# Copy configuration and profiles
COPY profiles/ /var/lib/hl7v2/profiles/
COPY schemas/ /var/lib/hl7v2/schemas/

# Set up directories
RUN mkdir -p /etc/hl7v2 /var/log/hl7v2 && \
    chown -R hl7v2:hl7v2 /var/lib/hl7v2 /var/log/hl7v2

# Switch to non-root user
USER hl7v2

# Expose ports
EXPOSE 8080 2575

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/hl7v2", "health"] || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/hl7v2"]
CMD ["server", "--config", "/etc/hl7v2/config.toml"]
