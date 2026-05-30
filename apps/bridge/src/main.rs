mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;

use anyhow::Result;
use axum::Router;
use deadpool_postgres::Pool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("antirot_bridge=info".parse()?),
        )
        .json()
        .init();

    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::migrate(&pool).await?;

    let bind = config.bind;
    let state = AppState { config, pool };
    let app = Router::new()
        .merge(routes::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(bind).await?;
    info!(%bind, "Antirot bridge listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
