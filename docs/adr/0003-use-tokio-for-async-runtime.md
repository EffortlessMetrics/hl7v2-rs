# ADR-0003: Use Tokio for Async Runtime

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Network module implementation requires async I/O

## Context

The hl7v2-rs server mode requires handling multiple concurrent connections for:
- MLLP over TCP (port 2575)
- HTTP API (port 8080)
- gRPC API (port 9090)

We need an async runtime that provides:
- Multi-threaded work-stealing scheduler
- Async TCP listeners and streams
- Timers for connection timeouts
- Channels for backpressure
- Wide ecosystem support
- Production-proven reliability

Available async runtimes in Rust:
1. **Tokio** - Most popular, comprehensive, battle-tested
2. **async-std** - Simpler API, smaller ecosystem
3. **smol** - Minimal, embeddable
4. **glommio** - Thread-per-core, specialized for high performance

## Decision

We will use **Tokio** as our async runtime with the following configuration:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec"] }
```

**Rationale:**
1. **Maturity** - Tokio is the most mature and widely used async runtime
2. **Ecosystem** - Axum (HTTP), Tonic (gRPC), rustls (TLS) all built on Tokio
3. **Performance** - Work-stealing scheduler handles varying workloads well
4. **Features** - Complete feature set (timers, signals, process, etc.)
5. **Community** - Large community, extensive documentation
6. **Production Use** - Used by Discord, AWS, Cloudflare, etc.

## Consequences

### Positive

- **Rich Ecosystem**: Can use Axum, Tonic, rustls without compatibility issues
- **Performance**: Work-stealing scheduler handles mixed I/O and CPU workloads
- **Reliability**: Battle-tested in production at massive scale
- **Features**: Built-in timers, channels, synchronization primitives
- **Documentation**: Excellent docs and tutorials
- **Tooling**: tokio-console for debugging, tracing integration

### Negative

- **Binary Size**: Tokio is larger than minimal runtimes like smol
- **Compile Time**: More features mean longer compile times
- **Learning Curve**: Async Rust + Tokio patterns have a learning curve
- **Opinionated**: Work-stealing vs thread-per-core (glommio approach)

### Neutral

- **Dependency**: We're now coupled to Tokio's ecosystem
- **Runtime Choice**: Other parts of the system must use Tokio or be compatible

## Alternatives Considered

### Alternative 1: async-std

**Pros:**
- Simpler API (mirrors std::net, std::fs)
- Smaller binary size
- Easier for beginners

**Cons:**
- Smaller ecosystem (fewer crates compatible)
- No Axum support (would need to use Tide)
- Less production usage data
- Development activity has slowed

**Why not chosen:**
Ecosystem incompatibility with Axum/Tonic. We'd have to use different HTTP/gRPC frameworks, fragmenting our stack.

### Alternative 2: smol

**Pros:**
- Minimal and embeddable
- Very small binary size
- Simple implementation

**Cons:**
- Much smaller ecosystem
- No Axum or Tonic support
- Would need to build more ourselves
- Less production proven

**Why not chosen:**
Too minimal for our needs. We'd spend time reimplementing features Tokio provides.

### Alternative 3: glommio

**Pros:**
- Thread-per-core architecture
- Extremely high performance for specific workloads
- Good for latency-sensitive applications

**Cons:**
- Very different programming model
- Much smaller ecosystem
- Linux-only (uses io_uring)
- Requires more careful workload partitioning

**Why not chosen:**
Cross-platform requirement (Linux, macOS, Windows). Thread-per-core model is overkill for our workload.

## Implementation Notes

### Tokio Configuration

For server mode, we'll use:

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Runtime configured via #[tokio::main] attribute
    run_server().await
}
```

For tests, we'll use:

```rust
#[tokio::test]
async fn test_mllp_server() {
    // Test code
}
```

### Runtime Parameters

We'll expose runtime tuning via environment variables:

- `TOKIO_WORKER_THREADS` - Number of worker threads (default: num_cpus)
- `TOKIO_BLOCKING_THREADS` - Blocking thread pool size (default: 512)

### Graceful Shutdown

Implement graceful shutdown using Tokio signals:

```rust
use tokio::signal;

async fn run_server() -> Result<(), Error> {
    let server = Server::new();

    tokio::select! {
        _ = server.run() => {},
        _ = signal::ctrl_c() => {
            println!("Shutting down gracefully...");
            server.shutdown().await?;
        }
    }

    Ok(())
}
```

### Backpressure with Channels

Use `tokio::sync::mpsc` for bounded channels:

```rust
use tokio::sync::mpsc;

let (tx, rx) = mpsc::channel(1024); // Capacity of 1024
```

When channel is full, sender will block (backpressure).

## References

- [Tokio Documentation](https://tokio.rs/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Tokio vs async-std comparison](https://www.arewewebyet.org/topics/async/)
- [Production Tokio Usage](https://tokio.rs/tokio/topics/production)
- [Axum on Tokio](https://github.com/tokio-rs/axum)
- [Tonic on Tokio](https://github.com/hyperium/tonic)
