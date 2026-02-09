//! A tiny example application that starts the QUIC + WebSocket server.
//!
//! This crate demonstrates how to add documentation comments in Rust
//! (the equivalent of JSDoc for JavaScript). Use `///` for item-level
//! docs and `//!` for crate/module-level docs. Generate HTML docs with:
//!
//! ```bash
//! cargo doc --open
//! ```

mod room;
mod server;

/// Program entry point — starts the server.
///
/// This function is the runtime entry and will call into the async
/// `run_server` implementation in the `server` module.
#[tokio::main]
async fn main() {
    server::run_server().await;
}
