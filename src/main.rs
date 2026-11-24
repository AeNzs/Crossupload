mod adapters;
mod db;
mod media;
mod routes;
mod security;
mod worker;

use anyhow::Result;
use db::Database;
use routes::{build_router, AppContext};
use security::AppConfig;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = AppConfig::from_env()?;
    let port = cfg.port;
    let app_url = cfg.app_url.clone();

    let db = Database::open(cfg)?;
    let http = adapters::build_http_client()?;

    worker::spawn_worker(db.clone(), http.clone());

    let router = build_router(AppContext { db, http });
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("====================================================");
    println!(" CrossUpload (Rust Single-Binary) siap dijalankan!");
    println!(" Web Studio : {app_url} (listening on 0.0.0.0:{port})");
    println!("====================================================");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
