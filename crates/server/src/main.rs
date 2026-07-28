use axum::{Router, routing::get};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // Assumes `trunk build` has already produced crates/web/dist; run from the
    // repo root (`cargo run -p server`) so this relative path resolves.
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .fallback_service(ServeDir::new("crates/web/dist"));

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
