#![allow(dead_code)]

pub mod routes;
pub mod state;
pub mod auth;
pub mod gc;

pub fn router() -> axum::Router {
    routes::router()
}

pub async fn serve(addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router()).await
}
