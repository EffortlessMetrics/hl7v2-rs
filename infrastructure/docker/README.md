# Docker Deployment

This directory contains the Docker image and Compose definition for running the
standalone `hl7v2-server` sidecar from a source checkout.

## Prerequisites

- Docker Engine with Docker Compose v2 (`docker compose`)
- A checkout of this repository
- Port `8080` available on the host

The Compose build context is the repository root because the image needs the
workspace source, profiles, and Cargo lockfile.

## Quick start

From the repository root:

```bash
docker compose -f infrastructure/docker/docker-compose.yml up --build -d
```

Check the service:

```bash
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/ready
```

The checked-in smoke proof exercises the running sidecar, including readiness,
redacted validation, evidence bundle creation, replay, and corpus diff:

```bash
python tests/server_smoke/smoke.py
```

Stop the service and remove its named volumes when the local evidence is no
longer needed:

```bash
docker compose -f infrastructure/docker/docker-compose.yml down -v
```

## Image build

Build the server image directly from the repository root:

```bash
docker build \
  --file infrastructure/docker/Dockerfile \
  --tag hl7v2-server:local \
  .
```

Run the image without Compose:

```bash
docker run --rm \
  --publish 8080:8080 \
  --env BIND_ADDRESS=0.0.0.0:8080 \
  --env HL7V2_API_KEY=dev-secret \
  hl7v2-server:local
```

The image runs as the non-root `hl7v2` user and includes a Docker healthcheck
for `/ready`. This direct invocation intentionally uses the image's built-in
defaults: it does not mount the checked-in server configuration or profiles and
does not provide persistent evidence-bundle or quarantine volumes. Use the
Compose workflow above for the configured readiness, validation, bundle/replay,
and corpus-diff smoke coverage.

## Compose service

The checked-in `docker-compose.yml` defines one service:

| Service | Purpose | Host port |
| --- | --- | --- |
| `hl7v2-server` | HTTP validation sidecar | `8080 -> 8080` |

The service uses the following container paths:

| Path | Source | Mode | Purpose |
| --- | --- | --- | --- |
| `/etc/hl7v2/server.toml` | `config/server.toml` | read-only bind mount | Local server configuration |
| `/var/lib/hl7v2/profiles` | `../../profiles` | read-only bind mount | Conformance profiles |
| `/var/lib/hl7v2/bundles` | `hl7v2-bundles` | named volume | Evidence bundle output |
| `/var/lib/hl7v2/quarantine` | `hl7v2-quarantine` | named volume | Quarantined artifacts |

The Compose healthcheck waits for `GET /ready`. Resource limits are set to
1 CPU and 512 MiB, with reservations of 0.25 CPU and 128 MiB.

## Configuration

The Compose file supplies local defaults:

| Variable | Compose value | Purpose |
| --- | --- | --- |
| `BIND_ADDRESS` | `0.0.0.0:8080` | Address and port inside the container |
| `HL7V2_CONFIG` | `/etc/hl7v2/server.toml` | Mounted server configuration |
| `HL7V2_API_KEY` | `${HL7V2_API_KEY:-dev-secret}` | API key for local requests |
| `HL7V2_PROFILE_PATHS` | `/var/lib/hl7v2/profiles/generic.yaml` | Profile search path |
| `RUST_LOG` | `hl7v2_server=info,tower_http=warn` | Log filtering |
| `RUST_LOG_FORMAT` | `json` | Structured JSON logs |

Override the local API key through the host environment:

```bash
HL7V2_API_KEY='replace-for-local-use' \
  docker compose -f infrastructure/docker/docker-compose.yml up --build -d
```

The checked-in `config/server.toml` configures the server’s bundle output and
quarantine roots inside the mounted named volumes. The
`sidecar.env.example` file contains matching local smoke-test defaults; it is
an example file and is not automatically loaded by Compose.

Do not use the development default or commit real deployment secrets for
shared or production environments.

## Smoke proof

With the service running, the smoke script uses:

- `HL7V2_SERVER_URL` to select the base URL (default:
  `http://127.0.0.1:8080`)
- `HL7V2_API_KEY` to authenticate requests (default: `dev-secret`)
- `HL7V2_SERVER_SMOKE_TIMEOUT` to control startup waiting (default: 45
  seconds)

For example:

```bash
HL7V2_SERVER_URL=http://127.0.0.1:8080 \
HL7V2_API_KEY='replace-for-local-use' \
python tests/server_smoke/smoke.py
```

The smoke script uses synthetic messages and checks that PHI sentinels do not
appear in the returned evidence responses. It is a local verification aid, not
a production deployment readiness claim.

## Related files

- [Dockerfile](Dockerfile) - multi-stage image build and runtime healthcheck
- [docker-compose.yml](docker-compose.yml) - local sidecar service definition
- [server.toml](config/server.toml) - mounted local server configuration
- [sidecar.env.example](sidecar.env.example) - local smoke-test environment example
- [Deployment Guide](../../DEPLOYMENT.md) - broader deployment guidance
- [Server smoke proof](../../tests/server_smoke/smoke.py) - executable local smoke check
