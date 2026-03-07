# Grafana Dashboards for HL7v2 Server

Production-ready Grafana dashboards for monitoring HL7v2 message processing, validation, and server performance.

## Dashboard Overview

### 1. HL7v2 Server - Overview (`hl7v2-server-overview.json`)

**Purpose**: Real-time operational monitoring of the HL7v2 HTTP API server

**Key Metrics**:
- **Request Rate**: Requests per second by endpoint and status code
- **Active Connections**: Current concurrent connections with thresholds
- **Request Latency**: P50/P95/P99 latency percentiles by endpoint
- **HTTP Status Codes**: 2xx (success), 4xx (client error), 5xx (server error) breakdown
- **Parse Errors**: Count of message parsing failures (last hour)
- **Memory Usage**: Resident memory (RSS) over time
- **CPU Usage**: CPU utilization percentage

**Use Cases**:
- Monitoring server health and performance
- Identifying latency issues
- Detecting error spikes
- Capacity planning

**Refresh Rate**: 10 seconds

### 2. HL7v2 - Validation Dashboard (`hl7v2-validation.json`)

**Purpose**: Message validation quality and conformance monitoring

**Key Metrics**:
- **Validation Results**: Valid vs invalid message counts (5-minute window)
- **Validation Success Rate**: Percentage gauge with color-coded thresholds
- **Top Profiles by Validation Rate**: Most frequently used conformance profiles
- **Validation Errors by Type**: Error categorization over time
- **Top 20 Validation Errors**: Table of most common errors with counts

**Use Cases**:
- Data quality monitoring
- Profile effectiveness analysis
- Error pattern identification
- Compliance reporting

**Refresh Rate**: 10 seconds

## Installation

### Docker Compose

The easiest way to get started with monitoring:

```yaml
# docker-compose.yml (monitoring stack)
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./infrastructure/grafana/prometheus.yml:/etc/prometheus/prometheus.yml
      - ./infrastructure/grafana/alerts:/etc/prometheus/alerts
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
    ports:
      - "9090:9090"
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    volumes:
      - ./infrastructure/grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./infrastructure/grafana/datasources:/etc/grafana/provisioning/datasources
      - grafana_data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_USER=admin
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-admin}
      - GF_INSTALL_PLUGINS=
    ports:
      - "3000:3000"
    depends_on:
      - prometheus
    restart: unless-stopped

  hl7v2-server:
    image: hl7v2-server:latest
    ports:
      - "8080:8080"
    environment:
      - HL7V2_HOST=0.0.0.0
      - HL7V2_PORT=8080
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  prometheus_data:
  grafana_data:
```

Start the monitoring stack:

```bash
export GRAFANA_ADMIN_PASSWORD="your-secure-password"
docker-compose up -d
```

Access:
- **Grafana**: http://localhost:3000 (admin / your-password)
- **Prometheus**: http://localhost:9090
- **HL7v2 Server**: http://localhost:8080

### Kubernetes

#### Step 1: Install Prometheus Operator

```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace \
  --set grafana.adminPassword=your-secure-password
```

#### Step 2: Deploy HL7v2 Server ServiceMonitor

```yaml
# infrastructure/k8s/servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: hl7v2-server
  namespace: hl7v2-system
  labels:
    app: hl7v2-server
spec:
  selector:
    matchLabels:
      app: hl7v2-server
  endpoints:
  - port: http
    path: /metrics
    interval: 10s
```

Apply:

```bash
kubectl apply -f infrastructure/k8s/servicemonitor.yaml
```

#### Step 3: Import Dashboards

Create ConfigMap for dashboard provisioning:

```bash
kubectl create configmap grafana-dashboard-hl7v2-overview \
  --from-file=infrastructure/grafana/dashboards/hl7v2-server-overview.json \
  --namespace=monitoring

kubectl create configmap grafana-dashboard-hl7v2-validation \
  --from-file=infrastructure/grafana/dashboards/hl7v2-validation.json \
  --namespace=monitoring

kubectl label configmap grafana-dashboard-hl7v2-overview grafana_dashboard=1 -n monitoring
kubectl label configmap grafana-dashboard-hl7v2-validation grafana_dashboard=1 -n monitoring
```

### Manual Import

1. Access Grafana UI
2. Navigate to Dashboards → Import
3. Upload JSON file or paste content
4. Select Prometheus datasource
5. Click Import

## Datasource Configuration

### Prometheus Datasource

Create `datasources/prometheus.yml`:

```yaml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: false
    jsonData:
      timeInterval: "15s"
      queryTimeout: "60s"
```

Place in `/etc/grafana/provisioning/datasources/` for automatic provisioning.

## Dashboard Provisioning

Create `dashboards/dashboards.yml`:

```yaml
apiVersion: 1

providers:
  - name: 'HL7v2 Dashboards'
    orgId: 1
    folder: 'HL7v2'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /etc/grafana/provisioning/dashboards
```

Place dashboard JSON files in the configured path.

## Metrics Reference

### Server Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `hl7v2_requests_total` | Counter | Total HTTP requests | endpoint, status |
| `hl7v2_request_duration_seconds` | Histogram | Request latency | endpoint |
| `hl7v2_active_connections` | Gauge | Current active connections | - |
| `hl7v2_parse_errors_total` | Counter | Parse error count | error_type |
| `hl7v2_validation_errors_total` | Counter | Validation error count | error_type |
| `process_resident_memory_bytes` | Gauge | Memory usage (RSS) | - |
| `process_cpu_seconds_total` | Counter | CPU time | - |

### Validation Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `hl7v2_validation_total` | Counter | Validation attempts | profile, result |
| `hl7v2_validation_errors_total` | Counter | Validation errors | profile, error_type |
| `hl7v2_profile_load_time_seconds` | Histogram | Profile load time | profile |

## Alerting

### Example Alert Rules

Create `alerts/hl7v2-server.yml`:

```yaml
groups:
  - name: hl7v2_server_alerts
    interval: 30s
    rules:
      # High error rate
      - alert: HL7v2HighErrorRate
        expr: |
          (
            sum(rate(hl7v2_requests_total{status=~"5.."}[5m]))
            /
            sum(rate(hl7v2_requests_total[5m]))
          ) > 0.05
        for: 5m
        labels:
          severity: warning
          component: hl7v2-server
        annotations:
          summary: "High error rate (>5%)"
          description: "HL7v2 server error rate is {{ $value | humanizePercentage }}"

      # High latency
      - alert: HL7v2HighLatency
        expr: |
          histogram_quantile(0.99,
            rate(hl7v2_request_duration_seconds_bucket[5m])
          ) > 0.1
        for: 5m
        labels:
          severity: warning
          component: hl7v2-server
        annotations:
          summary: "High request latency (P99 >100ms)"
          description: "P99 latency is {{ $value }}s"

      # Server down
      - alert: HL7v2ServerDown
        expr: up{job="hl7v2-server"} == 0
        for: 1m
        labels:
          severity: critical
          component: hl7v2-server
        annotations:
          summary: "HL7v2 server is down"
          description: "HL7v2 server has been down for 1 minute"

      # Low validation success rate
      - alert: HL7v2LowValidationSuccessRate
        expr: |
          (
            sum(rate(hl7v2_validation_total{result="valid"}[10m]))
            /
            sum(rate(hl7v2_validation_total[10m]))
          ) < 0.90
        for: 10m
        labels:
          severity: warning
          component: hl7v2-validation
        annotations:
          summary: "Low validation success rate (<90%)"
          description: "Validation success rate is {{ $value | humanizePercentage }}"
```

## Customization

### Adding Panels

1. Open dashboard in edit mode
2. Click "Add panel"
3. Configure query using PromQL
4. Set visualization type and options
5. Save dashboard

### Creating Variables

Example template variable for filtering by endpoint:

```json
{
  "name": "endpoint",
  "type": "query",
  "datasource": "Prometheus",
  "query": "label_values(hl7v2_requests_total, endpoint)",
  "multi": true,
  "includeAll": true
}
```

Use in queries: `hl7v2_requests_total{endpoint=~"$endpoint"}`

### Custom Time Ranges

Add to dashboard JSON:

```json
{
  "time": {
    "from": "now-6h",
    "to": "now"
  },
  "timepicker": {
    "refresh_intervals": ["5s", "10s", "30s", "1m", "5m"],
    "time_options": ["5m", "15m", "1h", "6h", "12h", "24h", "2d", "7d", "30d"]
  }
}
```

## Best Practices

### Dashboard Design

✅ **DO**:
- Use consistent color schemes across dashboards
- Set appropriate refresh rates (10s for operational, 1m for historical)
- Include thresholds on gauges for quick status assessment
- Use legends with calculated values (mean, max, current)
- Group related panels together
- Add panel descriptions for complex metrics

❌ **DON'T**:
- Overload dashboards with too many panels (≤12 recommended)
- Use overly aggressive refresh rates (<5s)
- Mix operational and historical views in same dashboard
- Use red/green colors only (consider colorblind accessibility)

### Metric Naming

Follow Prometheus naming conventions:
- Use snake_case: `hl7v2_requests_total`
- Suffix with unit: `_seconds`, `_bytes`, `_total`
- Use labels for dimensions: `{endpoint="/parse", status="200"}`

### Alert Tuning

- **for** clause: Wait period before firing (reduce flapping)
- **Severity levels**: critical (immediate action), warning (investigation needed), info (awareness)
- **Runbooks**: Link to troubleshooting docs in annotations

## Troubleshooting

### No Data in Dashboards

1. **Check Prometheus targets**:
   ```bash
   curl http://localhost:9090/api/v1/targets
   ```

2. **Verify metrics endpoint**:
   ```bash
   curl http://localhost:8080/metrics
   ```

3. **Check datasource configuration**: Grafana → Configuration → Data Sources

### High Memory Usage

- Reduce retention time in Prometheus
- Limit scrape frequency
- Use recording rules for expensive queries

### Slow Queries

- Optimize PromQL queries (avoid `rate(sum(...))`, use `sum(rate(...))`)
- Use recording rules for frequently used aggregations
- Increase Prometheus query timeout

## Related Documentation

- [DEPLOYMENT.md](../../DEPLOYMENT.md) - Server deployment guide
- [ADR-006](../../.qoder/adr/ADR-006-rate-limiting-and-backpressure.md) - Rate limiting strategy
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)

## Contributing

To add new dashboards:
1. Create dashboard in Grafana UI
2. Export JSON (Share → Export → Save to file)
3. Add to `infrastructure/grafana/dashboards/`
4. Document in this README
5. Submit PR

## License

These dashboards are part of the hl7v2-rs project and licensed under AGPL-3.0-or-later.
