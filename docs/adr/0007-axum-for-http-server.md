# ADR-0007: Axum for HTTP Server

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Server mode HTTP API implementation

## Context

We need an HTTP framework for the hl7v2-rs server mode that provides:

- RESTful API endpoints (`/hl7/parse`, `/hl7/validate`, `/hl7/ack`)
- JSON request/response handling
- Authentication middleware (Bearer tokens)
- Authorization middleware (RBAC)
- Request tracing and correlation IDs
- Graceful error handling
- Streaming requests/responses (for large messages)
- Integration with Prometheus metrics
- WebSocket support (future)

Available Rust HTTP frameworks:
1. **Axum** - Ergonomic, type-safe, built on Tower/Hyper
2. **Actix-web** - Mature, actor-based, high performance
3. **Rocket** - Simple, full-featured, macro-based
4. **Warp** - Filter-based, functional style
5. **Tide** - Simple async-std based framework

## Decision

We will use **Axum** for our HTTP server.

```toml
[dependencies]
axum = { version = "0.7", features = ["macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors", "compression"] }
```

**Rationale:**

1. **Type Safety** - Extractors are type-safe at compile time
2. **Tower Integration** - Leverages Tower middleware ecosystem
3. **Tokio Native** - Built on Tokio, matches our async runtime (ADR-0003)
4. **Ergonomic** - Excellent developer experience with minimal boilerplate
5. **Maintained** - Official Tokio project, active development
6. **Middleware** - Rich middleware ecosystem (tower, tower-http)
7. **Performance** - Built on Hyper, one of the fastest HTTP implementations

## Consequences

### Positive

- **Type Safety**: Extractors catch errors at compile time
- **Composability**: Tower middleware composes cleanly
- **Documentation**: Official Tokio project with excellent docs
- **Ecosystem**: Tower middleware works across Axum, Tonic, Hyper
- **Minimal Boilerplate**: No macros required (unlike Rocket)
- **Async Native**: Natural async/await support
- **Testability**: Easy to test handlers in isolation

### Negative

- **Learning Curve**: Tower middleware concepts take time to learn
- **Type Complexity**: Complicated type signatures in middleware
- **Newer**: Less battle-tested than Actix-web (but official Tokio project)
- **Breaking Changes**: Still evolving (0.x versions)

### Neutral

- **Hyper-based**: We're coupled to Hyper's HTTP implementation
- **Middleware Model**: Must think in Tower's middleware model

## Alternatives Considered

### Alternative 1: Actix-web

**Pros:**
- Most mature Rust web framework
- Extensive middleware ecosystem
- Very high performance benchmarks
- Large community

**Cons:**
- Actor-based model adds complexity
- Uses its own runtime (actix-rt based on Tokio)
- More boilerplate than Axum
- Macro-heavy

**Why not chosen:**
Actor model is overkill for our use case. We want simple request/response, not actor supervision trees. Axum's ergonomics are better.

### Alternative 2: Rocket

**Pros:**
- Very simple and ergonomic
- Excellent documentation
- Type-safe request guards
- Integrated templating

**Cons:**
- Macro-heavy (can slow compile times)
- Less flexible than Axum
- Smaller middleware ecosystem
- Custom attribute macros everywhere

**Why not chosen:**
Macros slow down compile times. Axum achieves similar ergonomics with plain functions.

### Alternative 3: Warp

**Pros:**
- Filter-based composition
- Type-safe like Axum
- Built on Hyper
- Good performance

**Cons:**
- Filter combinators are hard to read
- Type errors are cryptic
- Less ecosystem support than Axum
- Steeper learning curve

**Why not chosen:**
Axum's function-based handlers are more readable than Warp's filter chains. Axum is the successor to Warp (by same author).

### Alternative 4: Tide (async-std)

**Pros:**
- Simple and straightforward
- Middleware model is easy to understand
- Good for small projects

**Cons:**
- Built on async-std (we're using Tokio - ADR-0003)
- Smaller ecosystem
- Development has slowed
- Can't use Axum ecosystem

**Why not chosen:**
Incompatible with our Tokio choice. Would fragment our async stack.

## Implementation Notes

### Basic Server Setup

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Json},
    http::StatusCode,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    // Shared state
}

pub async fn run_server(config: ServerConfig) -> Result<(), Error> {
    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/hl7/parse", post(parse_handler))
        .route("/hl7/validate", post(validate_handler))
        .route("/hl7/ack", post(ack_handler))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### Handler Pattern

```rust
async fn parse_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ParsedMessage>, StatusCode> {
    match parse(body.as_bytes()) {
        Ok(msg) => Ok(Json(msg)),
        Err(e) => {
            tracing::error!("Parse error: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
```

### Middleware Stack

```rust
use tower::ServiceBuilder;
use tower_http::{trace, cors, compression, timeout};

let middleware = ServiceBuilder::new()
    .layer(trace::TraceLayer::new_for_http())
    .layer(cors::CorsLayer::permissive())
    .layer(compression::CompressionLayer::new())
    .layer(timeout::TimeoutLayer::new(Duration::from_secs(30)))
    .layer(auth::AuthLayer::new())  // Custom auth middleware
    .layer(rbac::RbacLayer::new()); // Custom RBAC middleware
```

### Authentication Middleware

```rust
use axum::{
    middleware::{self, Next},
    http::{Request, StatusCode},
    response::Response,
};

async fn auth_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(token) if token.starts_with("Bearer ") => {
            // Validate token
            next.run(req).await
        }
        _ => Err(StatusCode::UNAUTHORIZED)
    }
}
```

### Error Handling

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_json) = match self {
            Error::ParseError(e) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "code": "P_ParseError",
                    "message": e.to_string()
                })
            ),
            Error::ValidationError(e) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({
                    "code": "V_ValidationError",
                    "message": e.to_string()
                })
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "code": "S_InternalError",
                    "message": "Internal server error"
                })
            ),
        };

        (status, Json(error_json)).into_response()
    }
}
```

## References

- [Axum Documentation](https://docs.rs/axum/)
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- [Tower Middleware](https://docs.rs/tower/)
- [tower-http](https://docs.rs/tower-http/)
- [Hyper HTTP](https://hyper.rs/)
