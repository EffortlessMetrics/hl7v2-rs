# Deployment Guide

Comprehensive guide for deploying hl7v2-rs in production environments.

## Table of Contents

- [Quick Start](#quick-start)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Nix Deployment](#nix-deployment)
- [Configuration](#configuration)
- [Monitoring](#monitoring)
- [Security](#security)
- [Performance Tuning](#performance-tuning)
- [High Availability](#high-availability)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Local Development

```bash
# Clone the repository
git clone https://github.com/EffortlessMetrics/hl7v2-rs.git
cd hl7v2-rs

# Build and run
cargo run --bin hl7v2-server

# Server starts on http://localhost:8080
```

### Environment Variables

```bash
export HL7V2_HOST="0.0.0.0"              # Bind address (default: 127.0.0.1)
export HL7V2_PORT="8080"                  # Port (default: 8080)
export HL7V2_MAX_CONCURRENT="100"         # Max concurrent requests (default: 100)
export HL7V2_MAX_BODY_SIZE="1048576"      # Max body size in bytes (default: 1MB)
export HL7V2_API_KEY="your-secret-key"    # API key for authentication (optional)
export RUST_LOG="info"                    # Log level: trace, debug, info, warn, error
```

## Docker Deployment

### Using Pre-built Dockerfile

```bash
# Build Docker image
docker build -t hl7v2-server:latest .

# Run container
docker run -p 8080:8080 \
  -e HL7V2_HOST=0.0.0.0 \
  -e HL7V2_PORT=8080 \
  -e RUST_LOG=info \
  hl7v2-server:latest
```

### Using Nix-built Docker Image

```bash
# Build with Nix (reproducible, minimal image)
nix build .#docker
docker load < result

# Run
docker run -p 8080:8080 hl7v2-rs:latest
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  hl7v2-server:
    image: hl7v2-server:latest
    ports:
      - "8080:8080"
    environment:
      HL7V2_HOST: "0.0.0.0"
      HL7V2_PORT: "8080"
      HL7V2_MAX_CONCURRENT: "200"
      RUST_LOG: "info"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 128M

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    depends_on:
      - hl7v2-server

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    depends_on:
      - prometheus

volumes:
  prometheus_data:
  grafana_data:
```

Start the stack:

```bash
docker-compose up -d
```

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster (1.24+)
- kubectl configured
- Helm 3 (optional, for advanced deployment)

### Basic Deployment

Apply the manifests:

```bash
kubectl apply -f infrastructure/k8s/namespace.yaml
kubectl apply -f infrastructure/k8s/deployment.yaml
kubectl apply -f infrastructure/k8s/service.yaml
```

### With Ingress

```bash
kubectl apply -f infrastructure/k8s/ingress.yaml
```

### Full Stack with Monitoring

```bash
# Apply all manifests
kubectl apply -f infrastructure/k8s/

# Verify deployment
kubectl get pods -n hl7v2-system
kubectl get svc -n hl7v2-system

# Check logs
kubectl logs -n hl7v2-system -l app=hl7v2-server --tail=100 -f

# Port-forward for local testing
kubectl port-forward -n hl7v2-system svc/hl7v2-server 8080:80
```

### Customizing Deployment

Edit `infrastructure/k8s/deployment.yaml`:

```yaml
spec:
  replicas: 3  # Scale to 3 instances
  template:
    spec:
      containers:
      - name: hl7v2-server
        resources:
          requests:
            memory: "128Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
        env:
        - name: HL7V2_MAX_CONCURRENT
          value: "200"  # Increase concurrency limit
        - name: RUST_LOG
          value: "info"
```

Apply changes:

```bash
kubectl apply -f infrastructure/k8s/deployment.yaml
```

### Horizontal Pod Autoscaling

Create HPA:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: hl7v2-server-hpa
  namespace: hl7v2-system
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: hl7v2-server
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

Apply:

```bash
kubectl apply -f hpa.yaml
```

## Nix Deployment

### Using Nix Flakes

```bash
# Enter development environment
nix develop

# Build all packages
nix build

# Run server directly
nix run

# Build Docker image
nix build .#docker
docker load < result
```

### NixOS Module

For NixOS deployments, see [NIX_USAGE.md](NIX_USAGE.md) for complete guide.

## Configuration

### Server Configuration

**Bind Address and Port:**
```bash
HL7V2_HOST="0.0.0.0"  # Listen on all interfaces
HL7V2_PORT="8080"      # HTTP port
```

**Concurrency Limiting:**
```bash
HL7V2_MAX_CONCURRENT="100"  # Max concurrent requests (see ADR-006)
```

**Request Limits:**
```bash
HL7V2_MAX_BODY_SIZE="1048576"  # 1MB in bytes
```

**Authentication (Optional):**
```bash
HL7V2_API_KEY="your-secret-api-key-here"
```

When set, all requests must include `X-API-Key` header:

```bash
curl -H "X-API-Key: your-secret-api-key-here" http://localhost:8080/hl7/parse ...
```

### Logging Configuration

```bash
# Log levels: trace, debug, info, warn, error
export RUST_LOG="info"

# Module-specific logging
export RUST_LOG="hl7v2_server=debug,hl7v2_prof=info"

# JSON structured logging
export RUST_LOG_FORMAT="json"
```

### Configuration File

Create `config.yaml`:

```yaml
server:
  bind_address: "0.0.0.0:8080"
  max_body_size: 1048576
  max_concurrent: 100

logging:
  level: "info"
  format: "json"

security:
  api_key_required: true
  cors_allowed_origins:
    - "https://app.example.com"
    - "https://dashboard.example.com"
```

## Monitoring

### Prometheus Metrics

The server exposes metrics at `/metrics`:

**Key Metrics:**
- `hl7v2_requests_total` - Total request count by endpoint and status
- `hl7v2_request_duration_seconds` - Request latency histogram
- `hl7v2_active_connections` - Current active connections
- `hl7v2_parse_errors_total` - Parse error count
- `hl7v2_validation_errors_total` - Validation error count

**Prometheus Configuration:**

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'hl7v2-server'
    static_configs:
      - targets: ['hl7v2-server:8080']
    metrics_path: '/metrics'
```

### Grafana Dashboards

Import dashboards from `infrastructure/grafana/`:

1. **HL7v2 Server Overview**: Request rates, latencies, error rates
2. **HL7v2 Validation**: Validation success/failure rates, profile usage
3. **HL7v2 Performance**: P50/P95/P99 latencies, throughput

### Health Checks

**Health Endpoint:**
```bash
curl http://localhost:8080/health
# {"status":"healthy","uptime_seconds":1234}
```

**Readiness Endpoint:**
```bash
curl http://localhost:8080/ready
# {"ready":true}
```

**Kubernetes Probes:**

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 2
```

## Security

### API Key Authentication

Enable API key authentication:

```bash
export HL7V2_API_KEY="$(openssl rand -base64 32)"
```

Clients must include the key:

```bash
curl -H "X-API-Key: ${HL7V2_API_KEY}" http://localhost:8080/hl7/parse ...
```

### TLS/HTTPS

Use a reverse proxy (nginx, Traefik) for TLS termination:

**nginx example:**

```nginx
server {
    listen 443 ssl http2;
    server_name hl7.example.com;

    ssl_certificate /etc/ssl/certs/hl7.example.com.crt;
    ssl_certificate_key /etc/ssl/private/hl7.example.com.key;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {
        proxy_pass http://hl7v2-server:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Network Policies (Kubernetes)

Restrict network access:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: hl7v2-server-policy
  namespace: hl7v2-system
spec:
  podSelector:
    matchLabels:
      app: hl7v2-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: api-gateway
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: TCP
      port: 53  # DNS
```

### Security Best Practices

✅ **DO**:
- Use API key authentication in production
- Deploy behind HTTPS/TLS reverse proxy
- Implement rate limiting at API gateway
- Use network policies to restrict access
- Enable audit logging
- Rotate API keys regularly
- Monitor for security events

❌ **DON'T**:
- Expose server directly to public internet
- Use HTTP in production
- Share API keys across environments
- Disable CORS without good reason
- Run as root user

## Performance Tuning

### Concurrency Limit

Adjust based on capacity testing:

```bash
# Default: 100 concurrent requests
export HL7V2_MAX_CONCURRENT="200"
```

See [ADR-006](/.qoder/adr/ADR-006-rate-limiting-and-backpressure.md) for guidance.

### Resource Limits

**Kubernetes:**

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "250m"  # 0.25 CPU
  limits:
    memory: "512Mi"
    cpu: "1000m"  # 1 CPU
```

**Docker:**

```bash
docker run --cpus="1.0" --memory="512m" hl7v2-server:latest
```

### Benchmarking

Run performance tests:

```bash
# Install hey (HTTP load generator)
go install github.com/rakyll/hey@latest

# Benchmark parse endpoint
hey -n 10000 -c 100 -m POST \
  -H "Content-Type: application/json" \
  -d '{"message":"MSH|^~\\&|...","mllp_framed":false}' \
  http://localhost:8080/hl7/parse

# Expected results (modern CPU):
# Requests/sec: 5000-10000
# Latency P50: 5-10ms
# Latency P99: 20-50ms
```

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Request throughput | ≥5000 req/s | Parse endpoint, 200-byte messages |
| P50 latency | ≤10ms | Parse and validate |
| P99 latency | ≤50ms | Parse and validate |
| Memory per request | ≤1MB | Peak usage |
| Concurrent requests | ≥100 | With 1 CPU, 512MB RAM |

## High Availability

### Multi-Region Deployment

Deploy in multiple regions with global load balancer:

```
┌─────────────────┐
│ Global Load     │
│ Balancer (DNS)  │
└────────┬────────┘
         │
    ┌────┴─────┐
    │          │
┌───▼────┐ ┌──▼─────┐
│ US-East│ │ EU-West│
│ Region │ │ Region │
└────────┘ └────────┘
```

### Replication

Stateless design enables simple horizontal scaling:

```bash
# Kubernetes: Scale replicas
kubectl scale deployment hl7v2-server --replicas=5 -n hl7v2-system

# Verify
kubectl get pods -n hl7v2-system
```

### Load Balancing

**Layer 7 (HTTP) Load Balancing:**
- AWS Application Load Balancer
- GCP HTTP(S) Load Balancer
- Azure Application Gateway
- Kubernetes Ingress

**Layer 4 (TCP) Load Balancing:**
- AWS Network Load Balancer
- GCP Network Load Balancer
- HAProxy
- nginx (stream module)

### Disaster Recovery

**Backup:**
- No persistent state to backup
- Version control conformance profiles
- Document configuration as code

**Recovery:**
- Deploy new instances from container image
- Restore configuration from version control
- DNS failover to backup region

**RTO/RPO Targets:**
- Recovery Time Objective (RTO): <5 minutes
- Recovery Point Objective (RPO): 0 (no data loss - stateless)

## Troubleshooting

### Common Issues

**Server won't start:**

```bash
# Check port availability
lsof -i :8080

# Check environment variables
env | grep HL7V2

# Check logs
RUST_LOG=debug cargo run --bin hl7v2-server
```

**High latency:**

```bash
# Check metrics
curl http://localhost:8080/metrics | grep duration

# Check active connections
curl http://localhost:8080/metrics | grep active

# Check for 503 responses (concurrency limit hit)
curl http://localhost:8080/metrics | grep requests_total
```

**Parse errors:**

```bash
# Enable debug logging
RUST_LOG=hl7v2_core=debug cargo run --bin hl7v2-server

# Test with minimal message
curl -X POST http://localhost:8080/hl7/parse \
  -H "Content-Type: application/json" \
  -d '{"message":"MSH|^~\\&|A|B|C|D|20231119120000||ADT^A01|1|P|2.5\r"}'
```

**Memory issues:**

```bash
# Monitor memory usage
docker stats hl7v2-server

# Reduce max body size
export HL7V2_MAX_BODY_SIZE="524288"  # 512KB

# Reduce concurrency limit
export HL7V2_MAX_CONCURRENT="50"
```

### Debug Mode

Enable comprehensive logging:

```bash
export RUST_LOG="trace"
export RUST_BACKTRACE="full"
cargo run --bin hl7v2-server
```

### Support

- **Issues**: https://github.com/EffortlessMetrics/hl7v2-rs/issues
- **Documentation**: [README.md](README.md), [docs/STATUS.md](docs/STATUS.md)
- **Architecture Decisions**: [.qoder/adr/](/.qoder/adr/)
- **API Spec**: [schemas/openapi/hl7v2-api.yaml](schemas/openapi/hl7v2-api.yaml)

## Related Documentation

- [README.md](README.md) - Project overview and features
- [docs/STATUS.md](docs/STATUS.md) - Implementation roadmap
- [NIX_USAGE.md](NIX_USAGE.md) - Nix flake usage guide
- [OpenAPI Specification](schemas/openapi/hl7v2-api.yaml) - Complete API reference
- [Example Profiles](examples/profiles/README.md) - Conformance profile examples
- [ADR-006: Rate Limiting Strategy](/.qoder/adr/ADR-006-rate-limiting-and-backpressure.md)

## License

This deployment guide is part of the hl7v2-rs project and licensed under AGPL-3.0.
