//! Compile-time guard for the supported `hl7v2-server` Rust integration surface.

use std::sync::Arc;

use axum::Router;
use hl7v2_server::{AppState, Server, ServerBuilder, ServerConfig, build_router};

#[test]
fn supported_server_api_is_root_level_and_constructible() {
    let _: fn(Arc<AppState>) -> Router = build_router;
    let _: fn(ServerConfig) -> hl7v2_server::Result<Server> = Server::new;
    let _: fn() -> ServerBuilder = Server::builder;
}
